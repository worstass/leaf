use std::future::Future;
use std::io::Error;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::{sink::SinkExt, stream::StreamExt};
use log::*;
use protobuf::Message;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as TokioMutex;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::codec::Framed;

use crate::{
    app::dispatcher::Dispatcher,
    app::fake_dns::{FakeDns, FakeDnsMode},
    app::nat_manager::NatManager,
    config::{Inbound, PacketInboundSettings},
    option, Runner,
};

use crate::proxy::tun::netstack::NetStack;

use tokio::sync::mpsc::{Receiver as TokioReceiver, Sender as TokioSender};
use tokio::sync::mpsc::channel as tokio_channel;

use bytes::{BufMut, Bytes, BytesMut};
use std::net::{SocketAddr, TcpStream};
use std::pin::Pin;
use futures_util::stream::{Next, SplitStream};
use tokio::fs::File;
use tokio::net::{TcpListener, UdpSocket};
use tun::{AsyncDevice, TunPacket, TunPacketCodec};
use crate::app::nat_manager::UdpPacket;
use crate::config::packet_inbound_settings::Sink;
use crate::config::TunInboundSettings;

// use crate::config::PacketInboundSettings_Sink;
use crate::proxy::tun::netstack;
use crate::session::{DatagramSource, Network, Session, SocksAddr};

async fn handle_inbound_stream(
    stream: netstack::TcpStream,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    inbound_tag: String,
    dispatcher: Arc<Dispatcher>,
    fakedns: Arc<FakeDns>,
) {
    let mut sess = Session {
        network: Network::Tcp,
        source: local_addr,
        local_addr: remote_addr.clone(),
        destination: SocksAddr::Ip(remote_addr.clone()),
        inbound_tag: inbound_tag,
        ..Default::default()
    };
    // Whether to override the destination according to Fake DNS.
    if fakedns.is_fake_ip(&remote_addr.ip()).await {
        if let Some(domain) = fakedns.query_domain(&remote_addr.ip()).await {
            sess.destination = SocksAddr::Domain(domain, remote_addr.port());
        } else {
            // Although requests targeting fake IPs are assumed
            // never happen in real network traffic, which are
            // likely caused by poisoned DNS cache records, we
            // still have a chance to sniff the request domain
            // for TLS traffic in dispatcher.
            if remote_addr.port() != 443 {
                log::debug!(
                    "No paired domain found for this fake IP: {}, connection is rejected.",
                    &remote_addr.ip()
                );
                return;
            }
        }
    }
    dispatcher.dispatch_stream(sess, stream).await;
}

async fn handle_inbound_datagram(
    socket: Box<netstack::UdpSocket>,
    inbound_tag: String,
    nat_manager: Arc<NatManager>,
    fakedns: Arc<FakeDns>,
) {
    // The socket to receive/send packets from/to the netstack.
    let (ls, mut lr) = socket.split();
    let ls = Arc::new(ls);

    // The channel for sending back datagrams from NAT manager to netstack.
    let (l_tx, mut l_rx): (TokioSender<UdpPacket>, TokioReceiver<UdpPacket>) = tokio_channel(32);

    // Receive datagrams from NAT manager and send back to netstack.
    let fakedns_cloned = fakedns.clone();
    let ls_cloned = ls.clone();
    tokio::spawn(async move {
        while let Some(pkt) = l_rx.recv().await {
            let src_addr = match pkt.src_addr {
                SocksAddr::Ip(a) => a,
                SocksAddr::Domain(domain, port) => {
                    if let Some(ip) = fakedns_cloned.query_fake_ip(&domain).await {
                        SocketAddr::new(ip, port)
                    } else {
                        warn!(
                                "Received datagram with source address {}:{} without paired fake IP found.",
                                &domain, &port
                            );
                        continue;
                    }
                }
            };
            if let Err(e) = ls_cloned.send_to(&pkt.data[..], &src_addr, &pkt.dst_addr.must_ip()) {
                warn!("A packet failed to send to the netstack: {}", e);
            }
        }
    });

    // Accept datagrams from netstack and send to NAT manager.
    loop {
        match lr.recv_from().await {
            Err(e) => {
                log::warn!("Failed to accept a datagram from netstack: {}", e);
            }
            Ok((data, src_addr, dst_addr)) => {
                // Fake DNS logic.
                if dst_addr.port() == 53 {
                    match fakedns.generate_fake_response(&data).await {
                        Ok(resp) => {
                            if let Err(e) = ls.send_to(resp.as_ref(), &dst_addr, &src_addr) {
                                warn!("A packet failed to send to the netstack: {}", e);
                            }
                            continue;
                        }
                        Err(err) => {
                            trace!("generate fake ip failed: {}", err);
                        }
                    }
                }

                // Whether to override the destination according to Fake DNS.
                //
                // WARNING
                //
                // This allows datagram to have a domain name as destination,
                // but real UDP traffic are sent with IP address only. If the
                // outbound for this datagram is a direct one, the outbound
                // would resolve the domain to IP address before sending out
                // the datagram. If the outbound is a proxy one, it would
                // require a proxy server with the ability to handle datagrams
                // with domain name destination, leaf itself of course supports
                // this feature very well.
                let dst_addr = if fakedns.is_fake_ip(&dst_addr.ip()).await {
                    if let Some(domain) = fakedns.query_domain(&dst_addr.ip()).await {
                        SocksAddr::Domain(domain, dst_addr.port())
                    } else {
                        log::debug!(
                            "No paired domain found for this fake IP: {}, datagram is rejected.",
                            &dst_addr.ip()
                        );
                        continue;
                    }
                } else {
                    SocksAddr::Ip(dst_addr)
                };

                let dgram_src = DatagramSource::new(src_addr, None);
                let pkt = UdpPacket::new(data, SocksAddr::Ip(src_addr), dst_addr);
                nat_manager
                    .send(None, &dgram_src, &inbound_tag, &l_tx, pkt)
                    .await;
            }
        }
    }
}

pub fn new(
    inbound: Inbound,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
) -> Result<Runner> {
    let settings = PacketInboundSettings::parse_from_bytes(&inbound.settings)?;
    let port = settings.port;

    #[cfg(any(target_os = "linux"))]
    let pakcet_has_pi = settings.pi;
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    let pakcet_has_pi = false;
    #[cfg(any(target_os = "windows", target_os = "android"))]
    let pakcet_has_pi = false;

    // // FIXME it's a bad design to have 2 lists in config while we need only one
    let fake_dns_exclude = settings.fake_dns_exclude;
    let fake_dns_include = settings.fake_dns_include;
    if !fake_dns_exclude.is_empty() && !fake_dns_include.is_empty() {
        return Err(anyhow!(
            "fake DNS run in either include mode or exclude mode"
        ));
    }
    let (fake_dns_mode, fake_dns_filters) = if !fake_dns_include.is_empty() {
        (FakeDnsMode::Include, fake_dns_include)
    } else {
        (FakeDnsMode::Exclude, fake_dns_exclude)
    };
    let fakedns = Arc::new(FakeDns::new(fake_dns_mode));
    let mtu = settings.mtu;
    let runner: Runner = match settings.sink.unwrap() {
        Sink::TCP => {
            Box::pin(async move {
                let listen_addr = format!("127.0.0.1:{}", port);
                let listener = TcpListener::bind(&listen_addr).await.unwrap();
                info!("packet tcp inbound listening {}", &listen_addr);
                loop {
                    match listener.accept().await {
                        Ok((mut stream, _)) => {
                            let dispatcher = dispatcher.clone();
                            let nat_manager = nat_manager.clone();
                            let inbound_tag = inbound.clone().tag;
                            let (stack, mut tcp_listener, udp_socket) = NetStack::new();
                            let (mut stack_sink, mut stack_stream) = stack.split();
                            let mtu = 1512;
                            let (mut stream_reader, mut stream_writer) = stream.split();
                            let mut futs: Vec<Pin<Box<dyn Send + Future<Output=()>>>> = Vec::new();
                            futs.push(Box::pin(async move {
                                while let Some(pkt) = stack_stream.next().await {
                                    if let Ok(pkt) = pkt {
                                        trace!("stack->tcp:{:02X?}", &pkt[..]);
                                        match stream_writer.write(&pkt[..]).await {
                                            Ok(_) => (),
                                            Err(e) => {
                                                warn!("send pkt to tcp failed: {}", e);
                                                return;
                                            }
                                        }
                                    }
                                }
                            }));

                            futs.push(Box::pin(async move {
                                let mut packet_header_buf = vec![0; 6];
                                let mut packet_buf = vec![0u8; mtu];
                                loop {
                                    match stream_reader.read_exact(&mut packet_header_buf).await {
                                        Ok(n) => {
                                            if n < 6 {
                                                warn!("read <4 bytes, n={}", n);
                                                return;
                                            }
                                            let ver = packet_header_buf[0] >> 4;
                                            let pkt_len = match ver {
                                                4 => {
                                                    packet_header_buf[2] as usize * 256 + packet_header_buf[3] as usize
                                                }
                                                6 => {
                                                    let payload_len = packet_header_buf[4] as usize * 256 + packet_header_buf[5] as usize;
                                                    payload_len + 40
                                                }
                                                _ => {
                                                    warn!("packet ver wrong ver={}", ver);
                                                    return;
                                                }
                                            };
                                            let need_to_read = pkt_len - 6;
                                            match stream_reader.read_exact(&mut packet_buf[..need_to_read]).await {
                                                Ok(n) => {
                                                    if n < need_to_read {
                                                        warn!("read packet body <len={}", n);
                                                        return;
                                                    }
                                                    let pkt = [&packet_header_buf[..], &packet_buf[..n]].concat();
                                                    trace!("tcp->stack:{:02X?}", &pkt[..]);
                                                    match stack_sink.send(pkt).await {
                                                        Ok(_) => {}
                                                        Err(e) => {
                                                            warn!("write pkt to stack failed: {}", e);
                                                            return;
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    warn!("read packet body from tcp failed {:?}", e);
                                                    return;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!("read packet head from tcp failed {:?}", e);
                                            return;
                                        }
                                    }
                                }
                            }));

                            // Extracts TCP connections from stack and sends them to the dispatcher.
                            let inbound_tag_cloned = inbound_tag.clone();
                            let fakedns_cloned = fakedns.clone();
                            futs.push(Box::pin(async move {
                                while let Some((stream1, local_addr, remote_addr)) = tcp_listener.next().await {
                                    tokio::spawn(handle_inbound_stream(
                                        stream1,
                                        local_addr,
                                        remote_addr,
                                        inbound_tag_cloned.clone(),
                                        dispatcher.clone(),
                                        fakedns_cloned.clone(),
                                    ));
                                }
                            }));

                            // Receive and send UDP packets between netstack and NAT manager. The NAT
                            // manager would maintain UDP sessions and send them to the dispatcher.
                            let fakedns_cloned = fakedns.clone();
                            futs.push(Box::pin(async move {
                                handle_inbound_datagram(udp_socket, inbound_tag, nat_manager, fakedns_cloned.clone()).await
                            }));

                            info!("packet tcp inbound started");
                            futures::future::select_all(futs /*vec![handle_stream, handle_datagram]*/).await;
                            info!("packet tcp inbound exited");
                        }
                        Err(e) => {
                            error!("accept connection failed: {}", e);
                            break;
                        }
                    }
                }
            })
        }
        Sink::UDP => {
            Box::pin(async move {
                loop {
                    let dispatcher = dispatcher.clone();
                    let nat_manager = nat_manager.clone();
                    let inbound_tag = inbound.clone().tag;
                    let (stack, mut tcp_listener, udp_socket) = NetStack::new();
                    let (mut stack_sink, mut stack_stream) = stack.split();

                    let listen_addr = format!("127.0.0.1:{}", port);
                    let sock = UdpSocket::bind(&listen_addr).await.unwrap();
                    let remote_addr = format!("127.0.0.1:{}", port-1);
                    sock.connect(remote_addr).await.unwrap();
                    info!("packet udp inbound listening {}", &listen_addr);
                    let r = Arc::new(sock);
                    let s = r.clone();

                    let mut futs: Vec<Pin<Box<dyn Send + Future<Output=()>>>> = Vec::new();
                    futs.push(Box::pin(async move {
                        while let Some(pkt) = stack_stream.next().await {
                            if let Ok(pkt) = pkt {
                                trace!("stack->udp:{:02X?}", &pkt[..]);
                                match s.send(&pkt[..]).await {
                                    Ok(_) => (),
                                    Err(e) => {
                                        warn!("send pkt to udp failed: {}", e);
                                        // return;
                                    }
                                }
                            }
                        }
                    }));

                    futs.push(Box::pin(async move {
                        let mut packet_buf = vec![0u8; 1512];
                        loop {
                            match r.recv(&mut packet_buf).await {
                                Ok(n) => {
                                    let pkt = [&packet_buf[..n]].concat();
                                    trace!("udp->stack:{:02X?}", &pkt[..]);
                                    match stack_sink.send(pkt).await {
                                        Ok(_) => {}
                                        Err(e) => {
                                            warn!("udp write pkt to stack failed: {}", e);
                                            // return;
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("read packet head from udp failed {:?}", e);
                                    // return;
                                }
                            }
                        }
                    }));

                    // Extracts TCP connections from stack and sends them to the dispatcher.
                    let inbound_tag_cloned = inbound_tag.clone();
                    let fakedns_cloned = fakedns.clone();
                    futs.push(Box::pin(async move {
                        while let Some((stream1, local_addr, remote_addr)) = tcp_listener.next().await {
                            tokio::spawn(handle_inbound_stream(
                                stream1,
                                local_addr,
                                remote_addr,
                                inbound_tag_cloned.clone(),
                                dispatcher.clone(),
                                fakedns_cloned.clone(),
                            ));
                        }
                    }));

                    // Receive and send UDP packets between netstack and NAT manager. The NAT
                    // manager would maintain UDP sessions and send them to the dispatcher.
                    let fakedns_cloned = fakedns.clone();
                    futs.push(Box::pin(async move {
                        handle_inbound_datagram(udp_socket, inbound_tag, nat_manager, fakedns_cloned.clone()).await
                    }));
                    info!("packet udp inbound started");
                    futures::future::select_all(futs /*vec![handle_stream, handle_datagram]*/).await;
                    info!("packet udp inbound exited");
                }
            })
        }
        _ => {
            Box::pin(async move {})
        }
    };

    Ok(runner)
}