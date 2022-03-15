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
// use super::stack::NetStack;

use bytes::{BufMut, Bytes, BytesMut};
use std::net::{SocketAddr, TcpStream};
#[cfg(unix)]
use std::os::unix::io::FromRawFd;
use std::pin::Pin;
use tokio::fs::File;
use tokio::net::TcpListener;
use tun::{AsyncDevice, TunPacket, TunPacketCodec};
use crate::config::TunInboundSettings;

use crate::config::PacketInboundSettings_Sink;

pub fn new(
    inbound: Inbound,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
) -> Result<Runner> {
    let settings = PacketInboundSettings::parse_from_bytes(&inbound.settings)?;
    let port = settings.port;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let pakcet_has_pi = settings.pi;
    #[cfg(any(target_os = "ios"))]
    let pakcet_has_pi = true;
    #[cfg(any(target_os = "windows", target_os = "android"))]
    let pakcet_has_pi =  false;

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
    let mut fakedns = FakeDns::new(fake_dns_mode);
    let fakedns = Arc::new(TokioMutex::new(fakedns));

    Ok(Box::pin(async move {
        let listen_addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&listen_addr).await.unwrap();
        info!("packet inbound listening tcp {}", &listen_addr);
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let dispatcher = dispatcher.clone();
                    let nat_manager = nat_manager.clone();
                    let fakedns = fakedns.clone();
                    let tag = inbound.clone().tag;
                    let stack = NetStack::new(tag, dispatcher, nat_manager, fakedns);
                    let (mut stack_reader, mut stack_writer) = io::split(stack);
                    // let pi = has_packet_information();
                    let mtu = 1504;
                    let codec = TunPacketCodec::new(pakcet_has_pi, mtu);
                    let framed = Framed::new(stream, codec);
                    let (mut tun_sink, mut tun_stream) = framed.split();

                    let s2t = Box::pin(async move {
                        let mut buf = vec![0; mtu as usize];
                        loop {
                            match stack_reader.read(&mut buf).await {
                                Ok(0) => {
                                    debug!("read stack eof");
                                    return;
                                }
                                Ok(n) => {
                                    debug!("stack->tun:{:02X?}", &buf[..n]);
                                    match tun_sink.send(TunPacket::new((&buf[..n]).to_vec())).await {
                                        Ok(_) => (),
                                        Err(e) => {
                                            warn!("send pkt to tun failed: {}", e);
                                            return;
                                        }
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
                                Ok(packet) => {
                                    debug!("tun->stack:{:02X?}", packet.get_bytes());
                                    match stack_writer.write(packet.get_bytes()).await {
                                        Ok(_) => (),
                                        Err(e) => {
                                            warn!("write pkt to stack failed: {}", e);
                                            return;
                                        }
                                    }
                                },
                                Err(err) => {
                                    warn!("read tun failed {:?}", err);
                                    return;
                                }
                            }
                        }
                    });

                    info!("packet inbound started");
                    futures::future::select(t2s, s2t).await;
                    info!("packet inbound exited");
                }
                Err(e) => {
                    error!("accept connection failed: {}", e);
                    break;
                }
            }
        }
    }))
}