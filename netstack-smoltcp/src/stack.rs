use std::{
    cell::RefCell,
    rc::Rc,
    collections::HashMap,
    io,
    thread,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    os::raw,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll, Waker},
    collections::VecDeque,
    collections::BTreeMap,
};
use std::collections::btree_map::Entry;
use std::future::Future;
use std::sync::atomic::{AtomicI64, Ordering};

use futures::sink::{Sink};
use futures::stream::{Stream, StreamExt};

use smoltcp::{
    iface::{Interface, InterfaceBuilder, Routes, SocketHandle},
    phy,
    socket::{Socket, TcpSocket, TcpSocketBuffer, UdpPacketMetadata, UdpSocketBuffer},
    time::Instant,
    wire::{IpAddress, IpCidr, IpEndpoint, IpProtocol, Ipv4Address, Ipv4Packet, Ipv6Packet, TcpPacket, UdpPacket},
    phy::{Device, DeviceCapabilities, Medium, Checksum, ChecksumCapabilities},
    wire::Ipv6Address,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, ReadBuf},
    sync::Mutex as TokioMutex,
    sync::mpsc::{channel, Receiver, Sender},
};
use log::*;
use smoltcp::storage::RingBuffer;
use smoltcp::time::Duration;
use tokio::time::sleep_until;
use crate::{TcpListener, UdpSocket};

struct RxToken {
    buffer: Vec<u8>,
}

impl phy::RxToken for RxToken {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
        where
            F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        f.call_once((&mut self.buffer[..], ))
    }
}

struct TxToken<'a> {
    queue: &'a mut VecDeque<Vec<u8>>,
}

impl<'a> phy::TxToken for TxToken<'a> {
    fn consume<R, F>(mut self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
        where
            F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buf: Vec<u8> = Vec::new();
        buf.resize(len, 0);
        let result = f.call_once((&mut buf[..], ));
        self.queue.push_back(buf);
        result
    }
}

#[derive(Debug)]
struct Endpoint {
    inqueue: VecDeque<Vec<u8>>,
    outbuf: Vec<Vec<u8>>,
}

#[allow(clippy::new_without_default)]
impl Endpoint {
    pub fn new() -> Endpoint {
        Endpoint {
            inqueue: VecDeque::new(),
            outbuf: Vec::new(),
        }
    }

    pub fn recv_packet(self: &mut Self, buf: &[u8]) -> std::io::Result<()> {
        self.inqueue.push_back(Vec::from(buf));
        Ok(())
    }

    pub fn extract_packet(self: &mut Self, mut buf: &mut [u8]) -> Option<()> {
        match self.outbuf.pop() {
            None => None,
            Some(v) => {
                std::io::copy(&mut v.as_slice(), &mut buf);
                Some(())
            }
        }
    }
}

impl<'a> Device<'a> for Endpoint {
    type RxToken = RxToken;
    type TxToken = TxToken<'a>;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        self.inqueue.pop_front().map(move |buffer| {
            let rx = RxToken { buffer };
            let tx = TxToken {
                queue: &mut self.inqueue,
            };
            (rx, tx)
        })
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken {
            queue: &mut self.inqueue,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut cap = DeviceCapabilities::default();
        cap.medium = Medium::Ip;
        cap.max_transmission_unit = 1500;
        cap.checksum = ChecksumCapabilities::default();
        cap
    }
}

pub struct NetStack {
    waker: Option<Waker>,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
    sink_buf: Option<Vec<u8>>, // We're flushing per item, no need large buffer.

    tcp_sockets: BTreeMap<u16, SocketHandle>,
    // udp_sockets: BTreeMap<u16, Sender<(DestinationAddr, Vec<u8>)>>,

    iface: Arc<Mutex<Interface<'static, Endpoint>>>,
}

impl NetStack {
    pub fn new() -> (Self, TcpListener, Box<UdpSocket>) {
        Self::with_buffer_size(512, 64)
    }

    pub fn with_buffer_size(
        stack_buffer_size: usize,
        udp_buffer_size: usize,
    ) -> (Self, TcpListener, Box<UdpSocket>) {
        let endpoint = Endpoint::new();
        let ip_addrs = [
            IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24),
        ];

        // let mut routes_storage =[
        //     IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0),
        // ];

        let mut builder = InterfaceBuilder::new(endpoint, vec![])
            .any_ip(true)
            .ip_addrs(ip_addrs);
        // .routes(Routes::new(&mut routes_storage[..]));

        let mut iface = Arc::new(Mutex::new(builder.finalize()));
        let iface_clone = iface.clone();
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel(stack_buffer_size);

        tokio::spawn(async move {
            loop {
                let timestamp = Instant::now();
                let mut iface = iface.lock().unwrap();
                match iface.poll(timestamp) {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("poll error: {}", e);
                    }
                }
                for (h, sock) in iface.sockets() {
                    match sock {
                        Socket::Tcp(tcp) => {
                            if tcp.can_recv() {}
                            if tcp.can_send() {}
                        }
                        Socket::Udp(udp) => {
                            if udp.can_recv() {}
                            if udp.can_send() {}
                        }
                        _ => {}
                    }
                }
            }
        });
        (
            Self {
                waker: None,
                tx,
                rx,
                sink_buf: None,
                tcp_sockets: BTreeMap::new(),
                // udp_sockets: BTreeMap::new(),
                iface: iface_clone,
            },
            TcpListener::new(),
            UdpSocket::new(udp_buffer_size),
        )
    }

    fn check_packet(&mut self, pkt: &[u8]) {
        if pkt.len() < 20 { return; }

        // match packet[0] >> 4 {
        //     0b0100 => {
        //         let mut ipv4_packet = match Ipv4Packet::new_checked(pkt) {
        //             Ok(p) => p,
        //             Err(_) => return,
        //         };
        //         let (src_addr, dst_addr) = (ipv4_packet.src_addr(), ipv4_packet.dst_addr());
        //         match ipv4_packet.protocol() {
        //             Protocol::Tcp => {
        //                 let p = match TcpPacket::new_checked(ipv4_packet.payload_mut()) {
        //                     Ok(p) => p,
        //                     Err(_) => return,
        //                 };
        //                 let (src_port, dst_port, is_syn) = (p.src_port(), p.dst_port(), p.syn());
        //                 self.setup_tcp_socket(
        //                     smoltcp_addr_to_std(src_addr.into()),
        //                     dst_addr.into(),
        //                     src_port,
        //                     dst_port,
        //                     is_syn,
        //                     ipv4_packet.into_inner(),
        //                 )
        //             }
        //             Protocol::Udp => {
        //                 let mut p = match UdpPacket::new_checked(ipv4_packet.payload_mut()) {
        //                     Ok(p) => p,
        //                     Err(_) => return,
        //                 };
        //                 let (src_port, dst_port) = (p.src_port(), p.dst_port());
        //                 self.setup_udp_socket(
        //                     smoltcp_addr_to_std(src_addr.into()),
        //                     dst_addr.into(),
        //                     src_port,
        //                     dst_port,
        //                     p.payload_mut(),
        //                 );
        //             }
        //             _ => {}
        //         }
        //     }
        //     0b0110 => {
        //         // TODO: IPv6
        //     }
        //     _ => {}
        // }
        // match Ipv4Packet::new_checked(pkt) {
        //     Ok(packet) => {
        //         let src: Ipv4Addr = packet.src_addr().into();
        //         let dst: Ipv4Addr = packet.dst_addr().into();
        //         match packet.protocol() {
        //             smoltcp::wire::IpProtocol::Tcp => {
        //                 match TcpPacket::new_checked(pkt) {
        //                     Ok(p) => {
        //                         p.src_port();
        //                         p.dst_port();
        //                         let first = p.syn() && !p.ack();
        //                         if first {
        //                             let mut socket = TcpSocket::new(
        //                                 TcpSocketBuffer::new(vec![0; 4096]),
        //                                 TcpSocketBuffer::new(vec![0; 4096]));
        //                             socket.set_ack_delay(None);
        //                             socket.listen(IpEndpoint::new(packet.dst_addr().into(), p.dst_port())).unwrap();
        //                             let mut iface = self.iface.lock().unwrap();
        //                             let handle = iface.add_socket(socket);
        //                             let socket = iface.get_socket::<TcpSocket>(handle);
        //                             if socket.can_recv() {}
        //                         }
        //                     }
        //                     Err(_) => {}
        //                 }
        //             }
        //             smoltcp::wire::IpProtocol::Udp => {
        //                 //     match UdpPacket::new_checked(frame) {
        //                 //         Ok(p) => {
        //                 //             // Some(((result.src_port(), result.dst_port()), false, transport_offset + 8, packet.len() - 8))
        //                 //         }
        //                 //         Err(_) => {  },
        //                 //     }
        //             }
        //             _ => {}
        //         }
        //
        //
        //         //     if packet.protocol() == smoltcp::ip::Protocol::
        //         //     let proto: u8 = packet.protocol().into();
        //         //     let mut a: [u8; 4] = Default::default();
        //         //     a.copy_from_slice(packet.src_addr().as_bytes());
        //         //     let src_addr = IpAddr::from(a);
        //         //     a.copy_from_slice(packet.dst_addr().as_bytes());
        //         //     let dst_addr = IpAddr::from(a);
        //         //
        //         //     if let Some((ports, first_packet, payload_offset, payload_size))
        //         //     = get_transport_info(proto, packet.header_len().into(), &frame[packet.header_len().into()..]) {
        //         //         let connection = Connection {
        //         //             src: SocketAddr::new(src_addr, ports.0),
        //         //             dst: SocketAddr::new(dst_addr, ports.1),
        //         //             proto,
        //         //         };
        //         //         return Some((connection, first_packet, payload_offset, payload_size));
        //         //     } else {
        //         //         return None;
        //         //     }
        //     }
        //     _ => {}
        // }
        // match Ipv6Packet::new_checked(pkt) {
        //     Ok(packet) => {
        //         //         // TODO: Support extension headers.
        //         //         let proto: u8 = packet.next_header().into();
        //         //         let mut a: [u8; 16] = Default::default();
        //         //         a.copy_from_slice(packet.src_addr().as_bytes());
        //         //         let src_addr = IpAddr::from(a);
        //         //         a.copy_from_slice(packet.dst_addr().as_bytes());
        //         //         let dst_addr = IpAddr::from(a);
        //         //
        //         //         if let Some((ports, first_packet, payload_offset, payload_size))
        //         //         = get_transport_info(proto, packet.header_len().into(), &frame[packet.header_len().into()..]) {
        //         //             let connection = Connection {
        //         //                 src: SocketAddr::new(src_addr, ports.0),
        //         //                 dst: SocketAddr::new(dst_addr, ports.1),
        //         //                 proto,
        //         //             };
        //         //             Some((connection, first_packet, payload_offset, payload_size))
        //         //         } else {
        //         //             None
        //         //         }
        //     }
        //     _ => {}
        // }
    }

    // fn setup_udp_socket(&mut self: Self, src_addr: IpAddr,
    //                     dst_addr: smoltcp::wire::IpAddress,
    //                     src_port: u16,
    //                     dst_port: u16,
    //                     packet: &[u8], ) {
    //     let tx = match self.udp_sockets.entry(src_port) {
    //         Entry::Occupied(ent) => ent.into_mut(),
    //         Entry::Vacant(vac) => {
    //             // let next = match udp_next.upgrade() {
    //             //     Some(next) => next,
    //             //     None => return,
    //             // };
    //             // let (tx, rx) = bounded(48);
    //             // let stack_inner = stack.clone();
    //             // tokio::spawn(async move {
    //             //     next.on_session(
    //             //         Box::new(MultiplexedDatagramSessionAdapter::new(
    //             //             datagram::IpStackDatagramSession {
    //             //                 stack: stack_inner,
    //             //                 local_endpoint: src_addr.into(),
    //             //                 local_port: src_port,
    //             //             },
    //             //             rx.into_stream(),
    //             //             120,
    //             //         )),
    //             //         Box::new(FlowContext {
    //             //             local_peer: SocketAddr::new(src_addr, src_port),
    //             //             remote_peer: DestinationAddr {
    //             //                 host: HostName::Ip(smoltcp_addr_to_std(dst_addr)),
    //             //                 port: dst_port,
    //             //             },
    //             //         }),
    //             //     );
    //             // });
    //             vac.insert(tx)
    //         }
    //     };
    // }

    // fn setup_tcp_socket(&mut self: Self, src_addr: IpAddr,
    //                     dst_addr: smoltcp::wire::IpAddress,
    //                     src_port: u16,
    //                     dst_port: u16,
    //                     is_syn: bool,
    //                     packet: &[u8],
    // ) {
    //     let mut iface = self.iface.lock().unwrap();
    //     let dev = iface.device_mut();
    //     // dev.rx = Some(packet);
    //     let tcp_socket_count = self.tcp_sockets.len();
    //     if let Entry::Vacant(vac) = self.tcp_sockets.entry(src_port) {
    //         if !is_syn || tcp_socket_count >= 1 << 10 {
    //             return;
    //         }
    //         let mut socket = TcpSocket::new(
    //             // Note: The buffer sizes effectively affect overall throughput.
    //             RingBuffer::new(vec![0; 1024 * 14]),
    //             RingBuffer::new(vec![0; 10240]),
    //         );
    //         // This unwrap cannot panic for a valid TCP packet because:
    //         // 1) The socket is just created
    //         // 2) dst_port != 0
    //         socket.listen(IpEndpoint::new(dst_addr, dst_port)).unwrap();
    //         socket.set_nagle_enabled(false);
    //         // The default ACK delay (10ms) significantly reduces uplink throughput.
    //         // Maybe due to the delay when sending ACK packets?
    //         socket.set_ack_delay(None);
    //         let socket_handle = iface.add_socket(socket);
    //         vac.insert(socket_handle);
    //     }
    // }
}

impl Stream for NetStack {
    type Item = io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(pkt)) => Poll::Ready(Some(Ok(pkt))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl Sink<Vec<u8>> for NetStack {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.sink_buf.is_none() {
            Poll::Ready(Ok(()))
        } else {
            self.poll_flush(cx)
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        self.sink_buf.replace(item);
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Some(item) = self.sink_buf.take() {
            let mut iface = self.iface.lock().unwrap();
            let dev = iface.device_mut();
            // self.check_packet(&item[..]);
            match dev.recv_packet(&item[..]) {
                Ok(_) => {}
                Err(_) => {}
            }
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

// fn process_packet(stack: &IpStack, packet: &[u8]) {
//     if packet.len() < 20 {
//         return;
//     }
//     match packet[0] >> 4 {
//         0b0100 => {
//             let mut ipv4_packet = match Ipv4Packet::new_checked(packet) {
//                 Ok(p) => p,
//                 Err(_) => return,
//             };
//             let (src_addr, dst_addr) = (ipv4_packet.src_addr(), ipv4_packet.dst_addr());
//             match ipv4_packet.protocol() {
//                 IpProtocol::Tcp => {
//                     let p = match TcpPacket::new_checked(ipv4_packet.payload_mut()) {
//                         Ok(p) => p,
//                         Err(_) => return,
//                     };
//                     let (src_port, dst_port, is_syn) = (p.src_port(), p.dst_port(), p.syn());
//                     process_tcp(
//                         stack,
//                         smoltcp_addr_to_std(src_addr.into()),
//                         dst_addr.into(),
//                         src_port,
//                         dst_port,
//                         is_syn,
//                         ipv4_packet.into_inner(),
//                     );
//                 }
//                 IpProtocol::Udp => {
//                     let mut p = match UdpPacket::new_checked(ipv4_packet.payload_mut()) {
//                         Ok(p) => p,
//                         Err(_) => return,
//                     };
//                     let (src_port, dst_port) = (p.src_port(), p.dst_port());
//                     process_udp(
//                         stack,
//                         smoltcp_addr_to_std(src_addr.into()),
//                         dst_addr.into(),
//                         src_port,
//                         dst_port,
//                         p.payload_mut(),
//                     );
//                 }
//                 _ => {}
//             }
//         }
//         0b0110 => {
//             // TODO: IPv6
//         }
//         _ => {}
//     };
// }

fn smoltcp_addr_to_std(addr: IpAddress) -> IpAddr {
    match addr {
        IpAddress::Ipv4(ip) => IpAddr::V4(ip.into()),
        IpAddress::Ipv6(ip) => IpAddr::V6(ip.into()),
        _ => panic!("Cannot convert unknown smoltcp address to std address"),
    }
}

// fn process_tcp(
//     stack: &IpStack,
//     src_addr: IpAddr,
//     dst_addr: smoltcp::wire::IpAddress,
//     src_port: u16,
//     dst_port: u16,
//     is_syn: bool,
//     packet: &[u8],
// ) {
//     let mut guard = stack.lock().unwrap();
//     let IpStackInner {
//         netif,
//         tcp_sockets,
//         tcp_next,
//         ..
//     } = &mut *guard;
//
//     let dev = netif.device_mut();
//     dev.rx = Some(packet);
//
//     let tcp_socket_count = tcp_sockets.len();
//     if let Entry::Vacant(vac) = tcp_sockets.entry(src_port) {
//         if !is_syn || tcp_socket_count >= 1 << 10 {
//             return;
//         }
//         let next = match tcp_next.upgrade() {
//             Some(n) => n,
//             None => return,
//         };
//         let mut socket = TcpSocket::new(
//             // Note: The buffer sizes effectively affect overall throughput.
//             RingBuffer::new(vec![0; 1024 * 14]),
//             RingBuffer::new(vec![0; 10240]),
//         );
//         socket
//             .listen(IpEndpoint::new(dst_addr, dst_port))
//             // This unwrap cannot panic for a valid TCP packet because:
//             // 1) The socket is just created
//             // 2) dst_port != 0
//             .unwrap();
//         socket.set_nagle_enabled(false);
//         // The default ACK delay (10ms) significantly reduces uplink throughput.
//         // Maybe due to the delay when sending ACK packets?
//         socket.set_ack_delay(None);
//         let socket_handle = netif.add_socket(socket);
//         vac.insert(socket_handle);
//         let ctx = FlowContext {
//             local_peer: SocketAddr::new(src_addr, src_port),
//             remote_peer: DestinationAddr {
//                 host: HostName::Ip(smoltcp_addr_to_std(dst_addr)),
//                 port: dst_port,
//             },
//         };
//         tokio::spawn({
//             let stack = stack.clone();
//             async move {
//                 let mut stream = stream::IpStackStream {
//                     socket_entry: tcp_socket_entry::TcpSocketEntry {
//                         socket_handle,
//                         stack,
//                         local_port: src_port,
//                         most_recent_scheduled_poll: Arc::new(AtomicI64::new(i64::MAX)),
//                     },
//                     rx_buf: None,
//                     tx_buf: Some((Vec::with_capacity(4 * 1024), 0)),
//                 };
//                 if stream.handshake().await.is_ok() {
//                     next.on_stream(Box::new(stream) as _, Buffer::new(), Box::new(ctx));
//                 }
//             }
//         });
//     };
//     let now = Instant::now();
//     let _ = netif.poll(now.into());
//     // Polling the socket may wake a read/write waker. When a task polls the tx/rx
//     // buffer from the corresponding stream, a delayed poll will be rescheduled.
//     // Therefore, we don't have to poll the socket here.
// }
//
// fn process_udp(
//     stack: &IpStack,
//     src_addr: IpAddr,
//     dst_addr: smoltcp::wire::IpAddress,
//     src_port: u16,
//     dst_port: u16,
//     payload: &mut [u8],
// ) {
//     let mut guard = stack.lock().unwrap();
//     let IpStackInner {
//         udp_sockets,
//         udp_next,
//         ..
//     } = &mut *guard;
//     let tx = match udp_sockets.entry(src_port) {
//         Entry::Occupied(ent) => ent.into_mut(),
//         Entry::Vacant(vac) => {
//             let next = match udp_next.upgrade() {
//                 Some(next) => next,
//                 None => return,
//             };
//             let (tx, rx) = bounded(48);
//             let stack_inner = stack.clone();
//             tokio::spawn(async move {
//                 next.on_session(
//                     Box::new(MultiplexedDatagramSessionAdapter::new(
//                         datagram::IpStackDatagramSession {
//                             stack: stack_inner,
//                             local_endpoint: src_addr.into(),
//                             local_port: src_port,
//                         },
//                         rx.into_stream(),
//                         120,
//                     )),
//                     Box::new(FlowContext {
//                         local_peer: SocketAddr::new(src_addr, src_port),
//                         remote_peer: DestinationAddr {
//                             host: HostName::Ip(smoltcp_addr_to_std(dst_addr)),
//                             port: dst_port,
//                         },
//                     }),
//                 );
//             });
//             vac.insert(tx)
//         }
//     };
//     if let Err(TrySendError::Disconnected(_)) = tx.try_send((
//         DestinationAddr {
//             host: HostName::Ip(smoltcp_addr_to_std(dst_addr)),
//             port: dst_port,
//         },
//         payload.to_vec(),
//     )) {
//         udp_sockets.remove(&src_port);
//     }
//     // Drop packet when buffer is full
// }
//
// fn schedule_repoll(
//     stack: Arc<Mutex<IpStackInner>>,
//     poll_at: Instant,
//     most_recent_scheduled_poll: Arc<AtomicI64>,
// ) -> Pin<Box<dyn Future<Output=()> + Send + 'static>> {
//     debug_log(format!("Scheduled repoll: {:?}", poll_at));
//     let stack_cloned = stack.clone();
//     Box::pin(async move {
//         sleep_until(tokio::time::Instant::from_std(poll_at)).await;
//         if smoltcp::time::Instant::from(Instant::now()).total_millis()
//             > most_recent_scheduled_poll.load(Ordering::Relaxed)
//         {
//             // A more urgent poll was scheduled.
//             return;
//         }
//         let mut stack_guard = stack.lock().unwrap();
//         let _ = stack_guard.netif.poll(poll_at.into());
//         if let Some(delay) = stack_guard.netif.poll_delay(poll_at.into()) {
//             let scheduled_poll_milli =
//                 (smoltcp::time::Instant::from(Instant::now()) + delay).total_millis();
//             if scheduled_poll_milli >= most_recent_scheduled_poll.load(Ordering::Relaxed) {
//                 return;
//             }
//             // TODO: CAS spin loop
//             most_recent_scheduled_poll.store(scheduled_poll_milli, Ordering::Relaxed);
//
//             tokio::spawn(schedule_repoll(
//                 stack_cloned,
//                 poll_at + Duration::from(delay),
//                 most_recent_scheduled_poll,
//             ));
//         }
//     }) as _
// }