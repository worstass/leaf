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
use std::marker::PhantomData;
use std::sync::atomic::{AtomicI64, Ordering};

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
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, ReadBuf},
    sync::Mutex as TokioMutex,
    sync::mpsc::{channel, Receiver, Sender},
};
use log::*;
use smoltcp::iface::Route;
use smoltcp::storage::RingBuffer;
use smoltcp::time::Duration;
use smoltcp::wire::IpProtocol::Udp;
use tokio::time::sleep_until;
use crate::{TcpListener, TcpStream, UdpSocket};
use crate::tcp_listener::TcpListenerInner;
use crate::udp::UdpSocketInner;

#[derive(Debug, Clone)]
pub enum HostName {
    DomainName(String),
    Ip(IpAddr),
}

impl ToString for HostName {
    fn to_string(&self) -> String {
        match self {
            HostName::DomainName(s) => s.clone(),
            HostName::Ip(ip) => ip.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DestinationAddr {
    pub host: HostName,
    pub port: u16,
}

impl HostName {
    pub fn set_domain_name(&mut self, mut domain_name: String) -> Result<(), String> {
        // use trust_dns_resolver::Name;
        domain_name.make_ascii_lowercase();
        *self = HostName::DomainName(
            domain_name,
            // Name::from_utf8(&domain_name)
            //     .map_err(|_| domain_name)?
            //     .to_ascii(),
        );
        Ok(())
    }
    pub fn from_domain_name(domain_name: String) -> Result<Self, String> {
        let mut res = HostName::DomainName(String::new());
        res.set_domain_name(domain_name)?;
        Ok(res)
    }
}

impl From<SocketAddr> for DestinationAddr {
    fn from(socket: SocketAddr) -> Self {
        Self {
            host: HostName::Ip(socket.ip()),
            port: socket.port(),
        }
    }
}

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

struct TxToken {
// struct TxToken<'a> {
    // queue: &'a mut VecDeque<Vec<u8>>,
}

impl<'a> phy::TxToken for TxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
        where
            F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buf: Vec<u8> = Vec::new();
        buf.resize(len, 0);
        let result = f.call_once((&mut buf[..], ));
        // self.queue.push_back(buf);
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
    type TxToken = TxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        self.inqueue.pop_front().map(move |buffer| {
            let rx = RxToken { buffer };
            let tx = TxToken {
                // queue: &mut self.inqueue,
            };
            (rx, tx)
        })
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken {
            // queue: &mut self.inqueue,
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

pub(crate) struct NetStackInner {
    tcp_sockets: BTreeMap<u16, SocketHandle>,
    udp_sockets: BTreeMap<u16, flume::Sender<(DestinationAddr, Vec<u8>)>>,

    udp_socks: BTreeMap<u16, SocketHandle>,
}

pub struct NetStack<'a> {
    waker: Option<Waker>,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
    sink_buf: Option<Vec<u8>>, // We're flushing per item, no need large buffer.

    inner: Arc<Mutex<NetStackInner>>,
    // udp_sockets: BTreeMap<u16, Sender<(DestinationAddr, Vec<u8>)>>,

    // tcp_listener: TcpListener,

    iface: Arc<Mutex<Interface<'a, Endpoint>>>,
}

impl<'a> NetStack<'a> {
    pub fn new() -> (Self, TcpListener<'a>, Box<UdpSocket>) {
        Self::with_buffer_size(512, 64)
    }

    pub fn with_buffer_size(
        stack_buffer_size: usize,
        udp_buffer_size: usize,
    ) -> (Self, TcpListener<'a>, Box<UdpSocket>) {
        let endpoint = Endpoint::new();
        let ip_addrs = [
            IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24),
        ];
        let builder = InterfaceBuilder::new(endpoint, vec![])
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

        let inner = Arc::new(Mutex::new(NetStackInner {
            tcp_sockets: BTreeMap::new(),
            udp_sockets: BTreeMap::new(),

            udp_socks: BTreeMap::new(),
        }));

        let tcp_listener_inner = Arc::new(Mutex::new(TcpListenerInner::new()));

        let udp_socket_inner = Arc::new(UdpSocketInner::new(udp_buffer_size));

        let stack = Self {
            waker: None,
            tx,
            rx,
            sink_buf: None,
            inner: inner.clone(),
            // udp_sockets: BTreeMap::new(),
            // tcp_listener: TcpListener::new(tcp_listener_inner),
            iface: iface_clone,
        };
        let res = (
            stack,
            TcpListener::new(tcp_listener_inner.clone()),
            UdpSocket::new(udp_socket_inner, udp_buffer_size),
        );
        let stack_inner_clone = inner.clone();
        tokio::spawn(async move {
            loop {
                let timestamp = Instant::now();
                let mut iface = iface.lock().unwrap();
                // match iface.poll(timestamp) {
                //     Ok(_) => {}
                //     Err(e) => {
                //         debug!("smoltcp poll error: {}", e);
                //     }
                // }

                // let stack_inner = stack_inner_clone.lock().unwrap();
                //
                // for (port, handle) in stack_inner.tcp_sockets.iter() {
                //     let sock = iface.get_socket::<smoltcp::socket::TcpSocket>(*handle);
                //     if sock.may_send() {
                //         let tcp_s = TcpStream::new(sock);
                //         let mut tcp_inner = tcp_listener_inner.lock().unwrap();
                //         tcp_inner.add(tcp_s);
                //     }
                // }
            }
        });


        res
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
                        )
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

    fn setup_udp_socket(self: &mut Self,
                        src_addr: IpAddr,
                        dst_addr: smoltcp::wire::IpAddress,
                        src_port: u16,
                        dst_port: u16,
                        packet: &[u8], ) {
        let mut inner = self.inner.lock().unwrap();
        let tx = match inner.udp_sockets.entry(src_port) {
            Entry::Occupied(ent) => ent.into_mut(),
            Entry::Vacant(vac) => {
        //         // let next = match udp_next.upgrade() {
        //         //     Some(next) => next,
        //         //     None => return,
        //         // };
                let socket = smoltcp::socket::UdpSocket::new(
                    UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 64]),
                    UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 128]),
                );

                let mut iface = self.iface.lock().unwrap();
                let handle = iface.add_socket(socket);
                // vac.insert(handle);

                let buffer_size = 1024;
                let udp_inner = UdpSocketInner::new(buffer_size);
                UdpSocket::new(Arc::new(udp_inner),buffer_size);

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

    fn setup_tcp_socket(self: &mut Self,
                        src_addr: IpAddr,
                        dst_addr: smoltcp::wire::IpAddress,
                        src_port: u16,
                        dst_port: u16,
                        is_syn: bool,
                        packet: &[u8],
    ) {
        let mut iface = self.iface.lock().unwrap();
        let dev = iface.device_mut();
        // dev.rx = Some(packet);

        let mut inner = self.inner.lock().unwrap();
        let tcp_socket_count = inner.tcp_sockets.len();

        if let Entry::Vacant(vac) = inner.tcp_sockets.entry(src_port) {
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

impl<'a> Stream for NetStack<'a> {
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

impl<'a> Sink<Vec<u8>> for NetStack<'a> {
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


fn smoltcp_addr_to_std(addr: IpAddress) -> IpAddr {
    match addr {
        IpAddress::Ipv4(ip) => IpAddr::V4(ip.into()),
        IpAddress::Ipv6(ip) => IpAddr::V6(ip.into()),
        _ => panic!("Cannot convert unknown smoltcp address to std address"),
    }
}