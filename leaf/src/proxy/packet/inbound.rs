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

use bytes::{BufMut, Bytes, BytesMut};
use std::net::{SocketAddr, TcpStream};
#[cfg(unix)]
use std::os::unix::io::FromRawFd;
use std::pin::Pin;
use tokio::fs::File;
use tun::{AsyncDevice, TunPacket, TunPacketCodec};

use crate::config::PacketInboundSettings_Sink;
use crate::proxy::packet::{TcpSink, UdpSink};
use crate::proxy::packet::Sink;

#[cfg(unix)]
use crate::proxy::packet::FdSink;

#[cfg(unix)]
fn sink_from_fd(fd: i32) -> Result<Pin<Box<dyn Sink>>> {
    Ok(Box::pin(FdSink::new(fd)))
}

fn sink_from_udp(local_port: u32, remote_port: u32) -> Result<Pin<Box<dyn Sink>>>
{
    let local_addr: SocketAddr = format!("127.0.0.1:{}", local_port).parse()?;
    let remote_addr: SocketAddr = format!("127.0.0.1:{}", remote_port).parse()?;
    let sock = std::net::UdpSocket::bind(&local_addr)?;
    debug!("udp sink listen on {}", sock.local_addr()?);
    // sock.connect(remote_addr)?;
    Ok(Box::pin(UdpSink::new(sock)))
}

fn sink_from_tcp(port: u32) -> Result<Pin<Box<dyn Sink>>>
{
    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", port))?;
    let (stream, _) = listener.accept()?;
    debug!("tcp sink listen on {}", port);
    Ok(Box::pin(TcpSink::new(stream)))
}

impl Sink for AsyncDevice {}

fn sink_from_tun() -> Result<Pin<Box<dyn Sink>>>
{
    let mut config = tun::Configuration::default();
    config
        .address((10, 0, 0, 2))
        .netmask((255, 255, 255, 0))
        .destination((10, 0, 0, 1))
        .up();
    #[cfg(target_os = "linux")]
        config.platform(|config| {
        config.packet_information(true);
    });
    let dev = tun::create_as_async(&config).unwrap();
    Ok(Box::pin(dev))
}

impl Sink for File {}

#[cfg(unix)]
fn sink_from_pipe(pipe: &str) -> Result<Pin<Box<dyn Sink>>> {
    Ok(Box::pin(unsafe { File::from_raw_fd(21) }))
}

pub fn new(
    inbound: Inbound,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
) -> Result<Runner> {
    let settings = PacketInboundSettings::parse_from_bytes(&inbound.settings)?;

    Ok(Box::pin(async move {
        let fakedns = Arc::new(TokioMutex::new(FakeDns::new(FakeDnsMode::Exclude)));
        // for filter in fake_dns_filters.into_iter() {
        //     fakedns.lock().await.add_filter(filter);
        // }
        let stack = NetStack::new(inbound.tag.clone(), dispatcher, nat_manager, fakedns);
        info!("packet inbound started");
        loop {
            let mut tun = match settings.sink {
                #[cfg(unix)]
                PacketInboundSettings_Sink::FD =>{
                    debug!("using packet fd sink with fd={}", settings.fd);
                    Some(sink_from_fd(settings.fd).unwrap())
                } ,
                #[cfg(unix)]
                PacketInboundSettings_Sink::PIPE =>{
                    debug!("using packet pipe sink with pipe={}", settings.pipe);
                    Some(sink_from_pipe(settings.pipe.as_str()).unwrap())
                },
                PacketInboundSettings_Sink::UDP => {
                    debug!("using packet udp sink with ports: local={}, remote={}", settings.local_port, settings.remote_port);
                    // Some(sink_from_udp(settings.local_port, settings.remote_port).unwrap())
                    // Some(sink_from_tcp(settings.local_port).unwrap())
                    Some(sink_from_tun().unwrap())
                },
                #[cfg(not(unix))] _ => None,
            }.expect("Packet sink creation failed");
            let pi = false;
            let mtu = 1504;
            let codec = TunPacketCodec::new(pi, mtu);
            let framed = Framed::new(tun, codec);
            let (mut tun_sink, mut tun_stream) = framed.split();
            let (mut stack_reader, mut stack_writer) = io::split(stack);

            let s2t = Box::pin(async move {
                let mut buf = vec![0; mtu as usize];
                loop {
                    match stack_reader.read(&mut buf).await {
                        Ok(0) => {
                            debug!("read stack eof");
                            return;
                        }
                        Ok(n) => match tun_sink.send(TunPacket::new((&buf[..n]).to_vec())).await {
                            Ok(_) => (),
                            Err(e) => {
                                warn!("send pkt to tun failed: {}", e);
                                return;
                            }
                        },
                        Err(err) => {
                            warn!("read stack failed {:?}", err);
                            return;
                        }
                    }
                }
            });

            let t2s = Box::pin(async move {
                while let Some(packet) = tun_stream.next().await {
                    match packet {
                        Ok(packet) => match stack_writer.write(packet.get_bytes()).await {
                            Ok(_) => (),
                            Err(e) => {
                                warn!("write pkt to stack failed: {}", e);
                                return;
                            }
                        },
                        Err(err) => {
                            warn!("read tun failed {:?}", err);
                            return;
                        }
                    }
                }
            });

            futures::future::select(t2s, s2t).await;
            break;
        }
        info!("packet inbound exited");
    }))
}
