use std::io;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::ready;
use tokio::io::{AsyncRead, AsyncWrite, Interest, ReadBuf};
use std::net::{UdpSocket, SocketAddr};
use futures_util::future::ok;
use crate::proxy::packet::Sink;
use tokio_util::codec::Framed;
use tun::TunPacketCodec;
use crate::Runner;

pub struct UdpSink {
    inner: Box<tokio::net::UdpSocket>,
}

impl UdpSink {
    pub fn new(udp: std::net::UdpSocket) -> Self {
        let u = tokio::net::UdpSocket::from_std(udp).unwrap();
        Self {
            inner: Box::new(u),
        }
    }
}

impl Sink for UdpSink {}

impl AsyncRead for UdpSink {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let inner =  Pin::new(&mut self.inner);
        match inner.poll_recv_ready(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => inner.poll_recv(cx, buf),
        }
    }
}

impl AsyncWrite for UdpSink {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        let inner =  Pin::new(&mut self.inner);
        match inner.poll_send_ready(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => inner.poll_send(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

pub fn udp_runner() -> Runner {
    Box::pin(async move {
        let listen_addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&listen_addr).await.unwrap();
        info!("packet inbound listening tcp {}", &listen_addr);
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

                    info!("packet inbound started");
                    futures::future::select_all(futs /*vec![handle_stream, handle_datagram]*/).await;
                    info!("packet inbound exited");
                }
                Err(e) => {
                    error!("accept connection failed: {}", e);
                    break;
                }
            }
        }
    })
}