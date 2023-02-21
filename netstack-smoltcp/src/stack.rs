use std::{
    cell::RefCell,
    rc::Rc,
    collections::HashMap,
    io,
    thread,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    os::raw,
    pin::Pin,
    sync::{Arc},
    task::{Context, Poll, Waker},
    collections::VecDeque,
    collections::BTreeMap,
};
use std::collections::btree_map::Entry;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

use futures::sink::{Sink};
use futures::stream::{Stream, StreamExt};

use smoltcp::{
    iface::{Interface, InterfaceBuilder, Routes, SocketHandle},
    phy,
    socket::{Socket, TcpSocketBuffer, UdpPacketMetadata, UdpSocketBuffer},
    time::Instant,
    wire::{IpProtocol, IpAddress, IpCidr, IpEndpoint, Ipv4Address, Ipv4Packet, Ipv6Packet, TcpPacket, UdpPacket},
    phy::{Device, DeviceCapabilities, Medium, Checksum, ChecksumCapabilities},
    wire::Ipv6Address,
    iface::Route,
    storage::RingBuffer,
    time::Duration,
    wire::IpProtocol::Udp
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, ReadBuf},
    sync::mpsc::{channel, Receiver, Sender},
};
use log::*;
use tokio::time::sleep_until;
use crate::{DestinationAddr, Endpoint, TcpListener, TcpStream, UdpSocket};
use crate::tcp_listener::TcpListenerInner;
use crate::utils::*;

pub struct NetStack {
    sink_buf: Option<Vec<u8>>,
    // We're flushing per item, no need large buffer.
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
    waker: Option<Waker>,
    tcp_sockets: BTreeMap<u16, SocketHandle>,
    udp_sockets: BTreeMap<u16, flume::Sender<(DestinationAddr, Vec<u8>)>>,

    iface: Arc<Mutex<Interface<'static, Endpoint>>>,
    udp_socks: BTreeMap<u16, SocketHandle>,

    listener: Arc<Mutex<TcpListenerInner>>,
}

impl<'a> NetStack {
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
        let builder = InterfaceBuilder::new(endpoint, Vec::new())
            .any_ip(true)
            .ip_addrs(ip_addrs)
            .routes(Routes::new(
                [(
                    IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0),
                    // TODO: Custom IP Address
                    Route::new_ipv4_gateway(Ipv4Address::new(192, 168, 3, 1)),
                )]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
            ));
        let iface = Arc::new(Mutex::new(builder.finalize()));
        let iface_clone = iface.clone();
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel(stack_buffer_size);
        let listener_inner = Arc::new(Mutex::new(TcpListenerInner::new(iface_clone.clone())));

        let stack = Self {
            waker: None,
            tx,
            rx,
            sink_buf: None,
            tcp_sockets: BTreeMap::new(),
            udp_sockets: BTreeMap::new(),
            iface,
            udp_socks: BTreeMap::new(),
            listener: listener_inner.clone(),
        };

        // tokio::spawn(async move {
        //     loop {
        //         let mut iface = self.iface.lock().await;
        //         let timestamp = Instant::now();
        //         match iface.poll(timestamp) {
        //             Ok(_) => {}
        //             Err(e) => {
        //                 debug!("smoltcp poll error: {}", e);
        //             }
        //         }
        //
        //         for (_, handle) in self.tcp_sockets.iter() {
        //             let sock = iface.get_socket::<smoltcp::socket::TcpSocket>(*handle);
        //             if sock.may_send() {
        //                 let strea = TcpStream::new(*handle);
        //                 let mut lis = self.listener.lock().await;
        //                 lis.add(strea);
        //             }
        //         }
        //     }
        // });
        (
            stack,
            TcpListener::new(listener_inner.clone(), iface_clone.clone()),
            UdpSocket::new(iface_clone.clone(), udp_buffer_size),
        )
    }

    fn run(&self) {
        loop {
            let mut iface = self.iface.lock().unwrap();
            let timestamp = Instant::now();
            match iface.poll(timestamp) {
                Ok(_) => {}
                Err(e) => {
                    debug!("smoltcp poll error: {}", e);
                }
            }

            for (_, handle) in self.tcp_sockets.iter() {
                let sock = iface.get_socket::<smoltcp::socket::TcpSocket>(*handle);
                if sock.may_send() {
                    let strea = TcpStream::new(*handle, self.iface.clone());
                    let mut lis = self.listener.lock().unwrap();
                    lis.add(strea);
                }
            }
        }
    }

    fn check_packet(&mut self, pkt: &[u8]) {
        if pkt.len() < 20 { return; }

        match pkt[0] >> 4 {
            0b0100 => {
                let packet = match Ipv4Packet::new_checked(pkt) {
                    Ok(p) => p,
                    Err(_) => return,
                };
                let (src_addr, dst_addr) = (packet.src_addr(), packet.dst_addr());
                match packet.protocol() {
                    IpProtocol::Tcp => {
                        let p = match TcpPacket::new_checked(packet.payload()) {
                            Ok(p) => p,
                            Err(_) => return,
                        };
                        let (src_port, dst_port, is_syn) = (p.src_port(), p.dst_port(), p.syn());
                        self.setup_tcp_socket(
                            smoltcp_addr_to_std(src_addr.into()),
                            dst_addr.into(),
                            src_port,
                            dst_port,
                            is_syn,
                            packet.into_inner(),
                        );
                    }
                    IpProtocol::Udp => {
                        let p = match UdpPacket::new_checked(packet.payload()) {
                            Ok(p) => p,
                            Err(_) => return,
                        };
                        let (src_port, dst_port) = (p.src_port(), p.dst_port());
                        self.setup_udp_socket(
                            smoltcp_addr_to_std(src_addr.into()),
                            dst_addr.into(),
                            src_port,
                            dst_port,
                            p.payload(),
                        );
                    }
                    _ => {}
                }
            }
            0b0110 => {
                // TODO: IPv6
            }
            _ => {}
        }
    }

    fn setup_udp_socket(&mut self,
                        src_addr: IpAddr,
                        dst_addr: smoltcp::wire::IpAddress,
                        src_port: u16,
                        dst_port: u16,
                        packet: &[u8], ) {
        let tx = match self.udp_sockets.entry(src_port) {
            Entry::Occupied(ent) => ent.into_mut(),
            Entry::Vacant(vac) => {
                //         // let next = match udp_next.upgrade() {
                //         //     Some(next) => next,
                //         //     None => return,
                //         // };

                let (tx, rx) = flume::bounded(48);
                //         // let stack_inner = stack.clone();
                //         // tokio::spawn(async move {
                //         //     next.on_session(
                //         //         Box::new(MultiplexedDatagramSessionAdapter::new(
                //         //             datagram::IpStackDatagramSession {
                //         //                 stack: stack_inner,
                //         //                 local_endpoint: src_addr.into(),
                //         //                 local_port: src_port,
                //         //             },
                //         //             rx.into_stream(),
                //         //             120,
                //         //         )),
                //         //         Box::new(FlowContext {
                //         //             local_peer: SocketAddr::new(src_addr, src_port),
                //         //             remote_peer: DestinationAddr {
                //         //                 host: HostName::Ip(smoltcp_addr_to_std(dst_addr)),
                //         //                 port: dst_port,
                //         //             },
                //         //         }),
                //         //     );
                //         // });
                vac.insert(tx)
            }
        };
    }

    fn setup_tcp_socket(&mut self,
                        src_addr: IpAddr,
                        dst_addr: smoltcp::wire::IpAddress,
                        src_port: u16,
                        dst_port: u16,
                        is_syn: bool,
                        packet: &[u8],
    ) {
        let mut iface = self.iface.lock().unwrap();
        let tcp_socket_count = self.tcp_sockets.len();
        if let Entry::Vacant(vac) = self.tcp_sockets.entry(src_port) {
            if !is_syn || tcp_socket_count >= 1 << 10 {
                return;
            }
            let mut socket = smoltcp::socket::TcpSocket::new(
                // Note: The buffer sizes effectively affect overall throughput.
                RingBuffer::new(vec![0; 1024 * 14]),
                RingBuffer::new(vec![0; 10240]),
            );
            // This unwrap cannot panic for a valid TCP packet because:
            // 1) The socket is just created
            // 2) dst_port != 0
            socket.listen(IpEndpoint::new(dst_addr, dst_port)).unwrap();
            socket.set_nagle_enabled(false);
            // The default ACK delay (10ms) significantly reduces uplink throughput.
            // Maybe due to the delay when sending ACK packets?
            socket.set_ack_delay(None);
            let socket_handle = iface.add_socket(socket);
            vac.insert(socket_handle);
        }
    }
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
            self.check_packet(&item[..]);
            let mut iface = self.iface.lock().unwrap();
            let dev = iface.device_mut();
            match dev.recv_packet(&item[..]) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
