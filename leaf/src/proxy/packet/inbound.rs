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
    let mut fakedns = FakeDns::new(fake_dns_mode);
    let fakedns = Arc::new(TokioMutex::new(fakedns));

    Ok(Box::pin(async move {
        let listen_addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&listen_addr).await.unwrap();
        info!("packet inbound listening tcp {}", &listen_addr);
        // loop {
        //     match listener.accept().await {
        //         Ok((mut stream, _)) => {
        //             let dispatcher = dispatcher.clone();
        //             let nat_manager = nat_manager.clone();
        //             let fakedns = fakedns.clone();
        //             let tag = inbound.clone().tag;
        //             let stack = NetStack::new(tag, dispatcher, nat_manager, fakedns);
        //             let (mut stack_reader, mut stack_writer) = io::split(stack);
        //             // let pi = has_packet_information();
        //             let mtu = 1504;
        //             let (mut stream_reader, mut stream_writer) = stream.split();
        //             let s2t = Box::pin(async move {
        //                 let mut buf = vec![0; mtu as usize];
        //                 loop {
        //                     match stack_reader.read(&mut buf).await {
        //                         Ok(0) => {
        //                             debug!("read stack eof");
        //                             return;
        //                         }
        //                         Ok(n) => {
        //                             debug!("stack->tcp:{:02X?}", &buf[..n]);
        //                             match stream_writer.write(&buf[..n]).await {
        //                                 Ok(_) => (),
        //                                 Err(e) => {
        //                                     warn!("send pkt to tcp failed: {}", e);
        //                                     return;
        //                                 }
        //                             }
        //                         }
        //                         Err(err) => {
        //                             warn!("read stack failed {:?}", err);
        //                             return;
        //                         }
        //                     }
        //                 }
        //             });
        //             let t2s = Box::pin(async move {
        //                 let mut packet_header_buf = vec![0; 6];
        //                 let mut packet_buf = vec![0u8; mtu];
        //                 loop {
        //                     match stream_reader.read_exact(&mut packet_header_buf).await {
        //                         Ok(n) => {
        //                             if n < 6 {
        //                                 warn!("read <4 bytes, n={}", n);
        //                                 return;
        //                             }
        //                             let ver = packet_header_buf[0]>>4;
        //                             let pkt_len =  match ver {
        //                                 4=> {
        //                                     packet_header_buf[2] as usize * 256 + packet_header_buf[3] as usize
        //                                 },
        //                                 6=> {
        //                                     let payload_len = packet_header_buf[4] as usize * 256 + packet_header_buf[5] as usize;
        //                                     payload_len + 40
        //                                 }
        //                                 _=> {
        //                                     warn!("packet ver wrong ver={}", ver);
        //                                     return;
        //                                 }
        //                             };
        //                             let need_to_read = pkt_len-6;
        //                             match stream_reader.read_exact(&mut packet_buf[..need_to_read]).await {
        //                                 Ok(n) => {
        //                                     if n < need_to_read {
        //                                         warn!("read packet body <len={}", n);
        //                                         return;
        //                                     }
        //                                     let pkt = [&packet_header_buf[..], &packet_buf[..n]].concat();
        //                                     debug!("tcp->stack:{:02X?}", &pkt[..]);
        //                                     match stack_writer.write(&pkt[..]).await {
        //                                         Ok(_) => (),
        //                                         Err(e) => {
        //                                             warn!("write pkt to stack failed: {}", e);
        //                                             return;
        //                                         }
        //                                     }
        //                                 }
        //                                 Err(e) => {
        //                                     warn!("read packet body from tcp failed {:?}", e);
        //                                     return;
        //                                 }
        //                             }
        //                         }
        //                         Err(e) => {
        //                             warn!("read packet head from tcp failed {:?}", e);
        //                             return;
        //                         }
        //                     }
        //                 }
        //             });
        //
        //             info!("packet inbound started");
        //             futures::future::select(t2s, s2t).await;
        //             info!("packet inbound exited");
        //         }
        //         Err(e) => {
        //             error!("accept connection failed: {}", e);
        //             break;
        //         }
        //     }
        // }
    }))
}