use std::collections::HashMap;
use std::{io, thread};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use smoltcp::iface::{Interface, InterfaceBuilder, SocketHandle};
use smoltcp::socket::{Socket, TcpSocket, TcpSocketBuffer, UdpPacketMetadata, UdpSocket, UdpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{IpAddress, IpCidr, IpEndpoint, IpProtocol, Ipv4Packet, Ipv6Packet, TcpPacket, UdpPacket};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio::sync::Mutex as TokioMutex;
use log::*;
use tungstenite::Error::Protocol;

use crate::app::dispatcher::Dispatcher;
use crate::app::fake_dns::FakeDns;
use crate::app::nat_manager::NatManager;
use crate::session::{Network, Session, SocksAddr};
use super::endpoint::Endpoint;

pub struct NetStack<'a> {
    inbound_tag: String,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
    fakedns: Arc<TokioMutex<FakeDns>>,

    // connections: HashMap<Connection, ConnectionState>,
    // connection_managers: Vec<Box<dyn ConnectionManager>>,

    iface: Interface<'a, Endpoint>,
    // endpoint: Arc<Endpoint>,
}

impl<'a> NetStack<'a> {
    pub fn new(
        inbound_tag: String,
        dispatcher: Arc<Dispatcher>,
        nat_manager: Arc<NatManager>,
        fakedns: Arc<TokioMutex<FakeDns>>,
    ) -> Self {
        let endpoint = Endpoint::new();
        let ip_addrs = [
            IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24),
        ];
        let mut builder = InterfaceBuilder::new(endpoint, vec![])
            .any_ip(true)
            .ip_addrs(ip_addrs);
        let mut iface = builder.finalize();
        // let udp_rx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 64]);
        // let udp_tx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 128]);
        // let udp_socket = UdpSocket::new(udp_rx_buffer, udp_tx_buffer);
        // let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 64]);
        // let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 128]);
        // let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
        //
        // let udp_handle = iface.add_socket(udp_socket);
        // let tcp_handle = iface.add_socket(tcp_socket);
        //
        // let dispatcher = dispatcher.clone();
        //
        // tokio::spawn(async move {
        //     let tcp_socket = iface.get_socket::<TcpSocket>(tcp_handle);
        //     if !tcp_socket.is_open() {
        //         tcp_socket.listen(6969).unwrap();
        //     }
        // });

        let mut stack = Self {
            inbound_tag: inbound_tag.clone(),
            dispatcher,
            nat_manager,
            fakedns,
            // connection_managers: Vec::new(),
            // connections: HashMap::new(),
            iface,
        };
        stack
    }

    pub fn run(&mut self) {
        loop {
            let timestamp = Instant::now();
            println!("{}", self.inbound_tag);
            match self.iface.poll(timestamp) {
                Ok(_) => {}
                Err(e) => {
                    debug!("poll error: {}", e);
                }
            }
            for (h, sock) in self.iface.sockets() {
               match sock {
                   Socket::Tcp(tcp) => {
                       if tcp.can_recv() {

                       }
                       if tcp.can_send() {

                       }
                   }
                   Socket::Udp(udp) => {
                       if udp.can_recv() {

                       }
                       if udp.can_send() {

                       }
                   }
                   _ => {}
               }
            }
            // for (conn, value) in stack.connections.iter_mut() {
            //     let sock = iface.get_socket::<TcpSocket>(value.smoltcp_handle);
            // }
        }
    }

    fn check_packet(&mut self, frame: &[u8]) {
        match Ipv4Packet::new_checked(frame) {
            Ok(packet) => {
                let src: Ipv4Addr = packet.src_addr().into();
                let dst: Ipv4Addr = packet.dst_addr().into();
                match packet.protocol() {
                    smoltcp::wire::IpProtocol::Tcp => {
                        match TcpPacket::new_checked(frame) {
                            Ok(p) => {
                                p.src_port();
                                p.dst_port();
                                let first = p.syn() && !p.ack();
                                if first {
                                    let mut socket = TcpSocket::new(
                                        TcpSocketBuffer::new(vec![0; 4096]),
                                        TcpSocketBuffer::new(vec![0; 4096]));
                                    socket.set_ack_delay(None);
                                    socket.listen(IpEndpoint::new(packet.dst_addr().into(), p.dst_port())).unwrap();

                                    let handle = self.iface.add_socket(socket);
                                    let socket = self.iface.get_socket::<TcpSocket>(handle);
                                    if socket.can_recv() {}
                                }
                            }
                            Err(_) => {}
                        }
                    }
                    smoltcp::wire::IpProtocol::Udp => {
                        //     match UdpPacket::new_checked(frame) {
                        //         Ok(p) => {
                        //             // Some(((result.src_port(), result.dst_port()), false, transport_offset + 8, packet.len() - 8))
                        //         }
                        //         Err(_) => {  },
                        //     }
                    }
                    _ => {}
                }


                //     if packet.protocol() == smoltcp::ip::Protocol::
                //     let proto: u8 = packet.protocol().into();
                //     let mut a: [u8; 4] = Default::default();
                //     a.copy_from_slice(packet.src_addr().as_bytes());
                //     let src_addr = IpAddr::from(a);
                //     a.copy_from_slice(packet.dst_addr().as_bytes());
                //     let dst_addr = IpAddr::from(a);
                //
                //     if let Some((ports, first_packet, payload_offset, payload_size))
                //     = get_transport_info(proto, packet.header_len().into(), &frame[packet.header_len().into()..]) {
                //         let connection = Connection {
                //             src: SocketAddr::new(src_addr, ports.0),
                //             dst: SocketAddr::new(dst_addr, ports.1),
                //             proto,
                //         };
                //         return Some((connection, first_packet, payload_offset, payload_size));
                //     } else {
                //         return None;
                //     }
            }
            _ => {}
        }
        match Ipv6Packet::new_checked(frame) {
            Ok(packet) => {
                //         // TODO: Support extension headers.
                //         let proto: u8 = packet.next_header().into();
                //         let mut a: [u8; 16] = Default::default();
                //         a.copy_from_slice(packet.src_addr().as_bytes());
                //         let src_addr = IpAddr::from(a);
                //         a.copy_from_slice(packet.dst_addr().as_bytes());
                //         let dst_addr = IpAddr::from(a);
                //
                //         if let Some((ports, first_packet, payload_offset, payload_size))
                //         = get_transport_info(proto, packet.header_len().into(), &frame[packet.header_len().into()..]) {
                //             let connection = Connection {
                //                 src: SocketAddr::new(src_addr, ports.0),
                //                 dst: SocketAddr::new(dst_addr, ports.1),
                //                 proto,
                //             };
                //             Some((connection, first_packet, payload_offset, payload_size))
                //         } else {
                //             None
                //         }
            }
            _ => {}
        }
    }
    // fn new_udp_connection(&self,) {
    //     let dispatcher = self.dispatcher.clone();
    //
    //     let nat_manager = self.nat_manager.clone();
    //     let fakedns = self.fakedns.clone();
    //
    //     tokio::spawn(async move {
    //         let mut listener = UdpListener::new();
    //         let nat_manager = nat_manager.clone();
    //         let fakedns2 = fakedns.clone();
    //         let pcb = listener.pcb();
    //
    //         // Sending packets to TUN should be very fast.
    //         let (client_ch_tx, mut client_ch_rx): (
    //             TokioSender<UdpPacket>,
    //             TokioReceiver<UdpPacket>,
    //         ) = tokio_channel(32);
    //
    //         // downlink
    //         // let lwip_lock2 = lwip_lock.clone();
    //         tokio::spawn(async move {
    //             while let Some(pkt) = client_ch_rx.recv().await {
    //                 let socks_src_addr = match pkt.src_addr {
    //                     Some(a) => a,
    //                     None => {
    //                         warn!("unexpected none src addr");
    //                         continue;
    //                     }
    //                 };
    //                 let dst_addr = match pkt.dst_addr {
    //                     Some(a) => match a {
    //                         SocksAddr::Ip(a) => a,
    //                         _ => {
    //                             warn!("unexpected domain addr");
    //                             continue;
    //                         }
    //                     },
    //                     None => {
    //                         warn!("unexpected dst addr");
    //                         continue;
    //                     }
    //                 };
    //                 let src_addr = match socks_src_addr {
    //                     SocksAddr::Ip(a) => a,
    //
    //                     // If the socket gives us a domain source address,
    //                     // we assume there must be a paired fake IP, otherwise
    //                     // we have no idea how to deal with it.
    //                     SocksAddr::Domain(domain, port) => {
    //                         // TODO we're doing this for every packet! optimize needed
    //                         // trace!("downlink querying fake ip for domain {}", &domain);
    //                         if let Some(ip) = fakedns2.lock().await.query_fake_ip(&domain) {
    //                             SocketAddr::new(ip, port)
    //                         } else {
    //                             warn!(
    //                                 "unexpected domain src addr {}:{} without paired fake IP",
    //                                 &domain, &port
    //                             );
    //                             continue;
    //                         }
    //                     }
    //                 };
    //                 send_udp(lwip_lock2.clone(), &src_addr, &dst_addr, pcb, &pkt.data[..]);
    //             }
    //
    //             error!("unexpected udp downlink ended");
    //         });
    //
    //         let fakedns2 = fakedns.clone();
    //
    //         while let Some(pkt) = listener.next().await {
    //             let src_addr = match pkt.src_addr {
    //                 Some(a) => match a {
    //                     SocksAddr::Ip(a) => a,
    //                     _ => {
    //                         warn!("unexpected domain addr");
    //                         continue;
    //                     }
    //                 },
    //                 None => {
    //                     warn!("unexpected none src addr");
    //                     continue;
    //                 }
    //             };
    //             let dst_addr = match pkt.dst_addr {
    //                 Some(a) => match a {
    //                     SocksAddr::Ip(a) => a,
    //                     _ => {
    //                         warn!("unexpected domain addr");
    //                         continue;
    //                     }
    //                 },
    //                 None => {
    //                     warn!("unexpected dst addr");
    //                     continue;
    //                 }
    //             };
    //
    //             if dst_addr.port() == 53 {
    //                 match fakedns2.lock().await.generate_fake_response(&pkt.data) {
    //                     Ok(resp) => {
    //                         send_udp(lwip_lock.clone(), &dst_addr, &src_addr, pcb, resp.as_ref());
    //                         continue;
    //                     }
    //                     Err(err) => {
    //                         trace!("generate fake ip failed: {}", err);
    //                     }
    //                 }
    //             }
    //
    //             // We're sending UDP packets to a fake IP, and there should be a paired domain,
    //             // that said, the application connects a UDP socket with a domain address.
    //             // It also means the back packets on this UDP session shall only come from a
    //             // single source address.
    //             let socks_dst_addr = if fakedns2.lock().await.is_fake_ip(&dst_addr.ip()) {
    //                 // TODO we're doing this for every packet! optimize needed
    //                 // trace!("uplink querying domain for fake ip {}", &dst_addr.ip(),);
    //                 if let Some(domain) = fakedns2.lock().await.query_domain(&dst_addr.ip()) {
    //                     SocksAddr::Domain(domain, dst_addr.port())
    //                 } else {
    //                     // Skip this packet. Requests targeting fake IPs are
    //                     // assumed never happen in real network traffic.
    //                     continue;
    //                 }
    //             } else {
    //                 SocksAddr::Ip(dst_addr)
    //             };
    //
    //             let dgram_src = DatagramSource::new(src_addr, None);
    //
    //             let pkt = UdpPacket {
    //                 data: pkt.data,
    //                 src_addr: Some(SocksAddr::Ip(dgram_src.address)),
    //                 dst_addr: Some(socks_dst_addr.clone()),
    //             };
    //
    //             nat_manager
    //                 .send(&dgram_src, socks_dst_addr, &inbound_tag, pkt, &client_ch_tx)
    //                 .await;
    //         }
    //     });
    // }


    // fn new_tcp_connection(&self, ) {
    //     let dispatcher = self.dispatcher.clone();
    //     let fakedns =  self.fakedns.clone();
    //     let inbound_tag_1 =  self.inbound_tag.clone();
    //
    //     tokio::spawn(async move {
    //         let mut sess = Session {
    //             network: Network::Tcp,
    //             source: stream.local_addr().to_owned(),
    //             local_addr: stream.remote_addr().to_owned(),
    //             destination: SocksAddr::Ip(*stream.remote_addr()),
    //             inbound_tag: inbound_tag_1.clone(),
    //             ..Default::default()
    //         };
    //
    //         if fakedns.lock().await.is_fake_ip(&stream.remote_addr().ip()) {
    //             if let Some(domain) = fakedns
    //                 .lock()
    //                 .await
    //                 .query_domain(&stream.remote_addr().ip())
    //             {
    //                 sess.destination =
    //                     SocksAddr::Domain(domain, stream.remote_addr().port());
    //             } else {
    //                 // Although requests targeting fake IPs are assumed
    //                 // never happen in real network traffic, which are
    //                 // likely caused by poisoned DNS cache records, we
    //                 // still have a chance to sniff the request domain
    //                 // for TLS traffic in dispatcher.
    //                 if stream.remote_addr().port() != 443 {
    //                     return;
    //                 }
    //             }
    //         }
    //
    //         dispatcher
    //             .dispatch_tcp(&mut sess, TcpStream::new(stream))
    //             .await;
    //     });
    // }
    //
    // fn receive(&mut self, frame: &[u8]) {
    //     if let Some((connection, first_packet, _payload_offset, _payload_size)) = connection_tuple(frame) {
    //         if connection.proto == smoltcp::wire::IpProtocol::Tcp.into() {
    //             let cm = self.get_connection_manager(&connection);
    //             if cm.is_none() { return; }
    //             if first_packet {
    //                 for manager in self.connection_managers.iter_mut() {
    //                     if let Some(handler) = manager.new_connection(&connection) {
    //                         let mut socket =  TcpSocket::new(
    //                             TcpSocketBuffer::new(vec![0; 4096]),
    //                             TcpSocketBuffer::new(vec![0; 4096]));
    //                         socket.set_ack_delay(None);
    //                         socket.listen(connection.dst).unwrap();
    //                          // tokio::spawn(async move {
    //                          //     socket.accept
    //                          // });
    //                         let handle = self.iface.add_socket(socket);
    //                         let socket = self.iface.get_socket::<TcpSocket>(handle);
    //                         if socket.can_recv() {
    //
    //                         }
    //                         let mut state = ConnectionState {
    //                             smoltcp_handle: handle,
    //                             // mio_stream: client,
    //                             // token,
    //                             handler,
    //                         };
    //                         self.connections.insert(connection, state);
    //
    //                         // println!("[{:?}] CONNECT {}", chrono::offset::Local::now(), connection);
    //                         break;
    //                     }
    //                 }
    //             } else if !self.connections.contains_key(&connection) {
    //                 return;
    //             }
    //
    //             // Inject the packet to advance the smoltcp socket state
    //             self.iface.device_mut().inject_packet(frame);
    //
    //             // Read from the smoltcp socket and push the data to the connection handler.
    //             self.tunsocket_read_and_forward(&connection);
    //
    //             // The connection handler builds up the connection or encapsulates the data.
    //             // Therefore, we now expect it to write data to the server.
    //             self.write_to_server(&connection);
    //
    //         } else if connection.proto == smoltcp::wire::IpProtocol::Udp.into() {}
    //     }
    // }
    //
    // fn tunsocket_read_and_forward(&mut self, connection: &Connection) {
    //     if let Some(state) = self.connections.get_mut(&connection) {
    //         let closed = {
    //             let mut socket = self.iface.get::<TcpSocket>(state.smoltcp_handle).unwrap();
    //             let mut error = Ok(());
    //              while socket.can_recv() && error.is_ok() {
    //                 socket.recv(|data| {
    //                     let event = IncomingDataEvent {
    //                         direction: IncomingDirection::FromClient,
    //                         buffer: data,
    //
    //                     };
    //                     error = state.handler.push_data(event);
    //                     (data.len(), ())
    //                 }).unwrap();
    //             }
    //
    //             if error.is_err() {
    //                 // Self::print_error(error.unwrap_err());
    //                 true
    //             } else {
    //                 socket.state() == smoltcp::socket::TcpState::CloseWait
    //             }
    //         };
    //
    //         if closed {
    //             let connection_state = self.connections.get_mut(&connection).unwrap();
    //             // connection_state.mio_stream.shutdown(Shutdown::Both).unwrap();
    //             self.remove_connection(&connection);
    //             return;
    //         }
    //     }
    // }
    //
    // fn get_connection_manager(&self, connection: &Connection) -> Option<&Box<dyn ConnectionManager>>{
    //     for manager in self.connection_managers.iter() {
    //         if manager.handles_connection(connection) {
    //             return Some(manager);
    //         }
    //     }
    //     None
    // }
    //
    // fn expect_smoltcp_send(&mut self) {
    //     self.iface.poll( Instant::now()).unwrap();
    //
    //     while let Some(vec) = self.iface.device_mut().exfiltrate_packet() {
    //         let slice = vec.as_slice();
    //
    //         // TODO: Actual write. Replace.
    //         self.tun.transmit().unwrap().consume(Instant::now(), slice.len(), |buf| {
    //             buf[..].clone_from_slice(slice);
    //             Ok(())
    //         }).unwrap();
    //     }
    // }
    //
    // fn write_to_server(&mut self, connection: &Connection) {
    //     if let Some(state) = self.connections.get_mut(&connection) {
    //         let event = state.handler.peek_data(OutgoingDirection::ToServer);
    //         if event.buffer.len() == 0 {
    //             return;
    //         }
    //         let result = state.mio_stream.write(event.buffer);
    //         match result {
    //             Ok(consumed) => {
    //                 state.handler.consume_data(OutgoingDirection::ToServer, consumed);
    //             }
    //             Err(error) if error.kind() != std::io::ErrorKind::WouldBlock  => {
    //                 panic!("Error: {:?}", error);
    //             }
    //             _ => {
    //                 // println!("{:?}", result);
    //             }
    //         }
    //     }
    // }
    //
    // fn write_to_client(&mut self, connection: &Connection) {
    //     if let Some(state) = self.connections.get_mut(&connection) {
    //         let event = state.handler.peek_data(OutgoingDirection::ToClient);
    //         let socket = &mut self.iface.get::<TcpSocket>(state.smoltcp_handle).unwrap();
    //         if socket.may_send() {
    //             let consumed = socket.send_slice(event.buffer).unwrap();
    //             state.handler.consume_data(OutgoingDirection::ToClient, consumed);
    //         }
    //     }
    // }
}

impl<'a> Drop for NetStack<'a> {
    fn drop(&mut self) {}
}

impl<'a> AsyncRead for NetStack<'a> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        let rbuf = buf.initialize_unfilled();
        match self.iface.device_mut().extract_packet(rbuf) {
            Some(e) => Poll::Ready(Ok(())),
            None => {
                let waker = cx.waker().clone();
                thread::spawn(move || {
                    waker.wake();
                });
                Poll::Pending
            }
        }
    }
}

impl<'a> AsyncWrite for NetStack<'a> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.check_packet(buf);
        self.iface.device_mut().inject_packet(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Hash, Clone, Copy)]
pub struct Connection {
    pub src: std::net::SocketAddr,
    pub dst: std::net::SocketAddr,
    pub proto: u8,
}

pub(crate) trait TcpProxy {}

pub(crate) trait ConnectionManager {
    fn handles_connection(&self, connection: &Connection) -> bool;
    fn new_connection(&mut self, connection: &Connection) -> Option<std::boxed::Box<dyn TcpProxy>>;
    fn close_connection(&mut self, connection: &Connection);
    fn get_server(&self) -> SocketAddr;
}

struct ConnectionState {
    smoltcp_handle: SocketHandle,
    // mio_stream: TcpStream,
    // token: Token,
    handler: Box<dyn TcpProxy>,
}


fn get_transport_info(proto: u8, transport_offset: usize, packet: &[u8]) -> Option<((u16, u16), bool, usize, usize)> {
    if IpProtocol::from(proto) == smoltcp::wire::IpProtocol::Udp.into() {
        match UdpPacket::new_checked(packet) {
            Ok(result) => {
                Some(((result.src_port(), result.dst_port()), false, transport_offset + 8, packet.len() - 8))
            }
            Err(_) => None
        }
    } else if IpProtocol::from(proto) == smoltcp::wire::IpProtocol::Tcp.into() {
        match TcpPacket::new_checked(packet) {
            Ok(result) => {
                Some(((result.src_port(), result.dst_port()), result.syn() && !result.ack(),
                      transport_offset + result.header_len() as usize, packet.len()))
            }
            Err(_) => None
        }
    } else {
        None
    }
}

fn connection_tuple(frame: &[u8]) -> Option<(Connection, bool, usize, usize)> {
    match Ipv4Packet::new_checked(frame) {
        Ok(packet) => {
            let proto: u8 = packet.protocol().into();
            let mut a: [u8; 4] = Default::default();
            a.copy_from_slice(packet.src_addr().as_bytes());
            let src_addr = IpAddr::from(a);
            a.copy_from_slice(packet.dst_addr().as_bytes());
            let dst_addr = IpAddr::from(a);

            if let Some((ports, first_packet, payload_offset, payload_size))
            = get_transport_info(proto, packet.header_len().into(), &frame[packet.header_len().into()..]) {
                let connection = Connection {
                    src: SocketAddr::new(src_addr, ports.0),
                    dst: SocketAddr::new(dst_addr, ports.1),
                    proto,
                };
                return Some((connection, first_packet, payload_offset, payload_size));
            } else {
                return None;
            }
        }
        _ => {}
    }

    match Ipv6Packet::new_checked(frame) {
        Ok(packet) => {
            // TODO: Support extension headers.
            let proto: u8 = packet.next_header().into();
            let mut a: [u8; 16] = Default::default();
            a.copy_from_slice(packet.src_addr().as_bytes());
            let src_addr = IpAddr::from(a);
            a.copy_from_slice(packet.dst_addr().as_bytes());
            let dst_addr = IpAddr::from(a);

            if let Some((ports, first_packet, payload_offset, payload_size))
            = get_transport_info(proto, packet.header_len().into(), &frame[packet.header_len().into()..]) {
                let connection = Connection {
                    src: SocketAddr::new(src_addr, ports.0),
                    dst: SocketAddr::new(dst_addr, ports.1),
                    proto,
                };
                Some((connection, first_packet, payload_offset, payload_size))
            } else {
                None
            }
        }
        _ => None
    }
}