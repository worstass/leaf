use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::{sink::SinkExt, stream::StreamExt};
use log::*;
use protobuf::Message;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as TokioMutex;

use crate::{
    app::dispatcher::Dispatcher,
    app::fake_dns::{FakeDns, FakeDnsMode},
    app::nat_manager::NatManager,
    config::{Inbound},
    option, Runner,
    proxy::tun::netstack::NetStack,
};

use bytes::{BufMut, Bytes, BytesMut};

/// A packet protocol IP version
#[derive(Debug)]
enum PacketProtocol {
    IPv4,
    IPv6,
    Other(u8),
}

// Note: the protocol in the packet information header is platform dependent.
impl PacketProtocol {
    #[cfg(target_os = "linux")]
    fn into_pi_field(&self) -> Result<u16, io::Error> {
        match self {
            PacketProtocol::IPv4 => Ok(libc::ETH_P_IP as u16),
            PacketProtocol::IPv6 => Ok(libc::ETH_P_IPV6 as u16),
            PacketProtocol::Other(_) => Err(io::Error::new(
                io::ErrorKind::Other,
                "neither an IPv4 or IPv6 packet",
            )),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn into_pi_field(&self) -> Result<u16, io::Error> {
        match self {
            PacketProtocol::IPv4 => Ok(libc::PF_INET as u16),
            PacketProtocol::IPv6 => Ok(libc::PF_INET6 as u16),
            PacketProtocol::Other(_) => Err(io::Error::new(
                io::ErrorKind::Other,
                "neither an IPv4 or IPv6 packet",
            )),
        }
    }
}

// use tun::async::codec::PacketProtocol;
/// A Packet to be sent or received on the TUN interface.
#[derive(Debug)]
pub struct Packet(PacketProtocol, Bytes);
// const MTU: usize = 1500;

/// Infer the protocol based on the first nibble in the packet buffer.
fn infer_proto(buf: &[u8]) -> PacketProtocol {
    match buf[0] >> 4 {
        4 => PacketProtocol::IPv4,
        6 => PacketProtocol::IPv6,
        p => PacketProtocol::Other(p),
    }
}

impl Packet {
    /// Create a new `Packet` based on a byte slice.
    pub fn new(bytes: Vec<u8>) -> Packet {
        let proto = infer_proto(&bytes);
        Packet(proto, Bytes::from(bytes))
    }

    /// Return this packet's bytes.
    pub fn get_bytes(&self) -> &[u8] {
        &self.1
    }
}

pub fn new(
    inbound: Inbound,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
) -> Result<Runner> {
    unimplemented!();
    // Ok(Box::pin(async move {
    //     let framed = pkt.into_framed();
    //     let (mut packet_sink, mut packet_stream) = framed.split();
    //     let stack = NetStack::new(inbound.tag.clone(), dispatcher, nat_manager, fakedns);
    //     let (mut stack_reader, mut stack_writer) = io::split(stack);
    //
    //     let s2t = Box::pin(async move {
    //         let mut buf = vec![0; mtu as usize];
    //         loop {
    //             match stack_reader.read(&mut buf).await {
    //                 Ok(0) => {
    //                     debug!("read stack eof");
    //                     return;
    //                 }
    //                 Ok(n) => match packet_sink.send(Packet::new((&buf[..n]).to_vec())).await {
    //                     Ok(_) => (),
    //                     Err(e) => {
    //                         warn!("send pkt to tun failed: {}", e);
    //                         return;
    //                     }
    //                 },
    //                 Err(err) => {
    //                     warn!("read stack failed {:?}", err);
    //                     return;
    //                 }
    //             }
    //         }
    //     });
    //     let t2s = Box::pin(async move {
    //         while let Some(packet) = packet_stream.next().await {
    //             match packet {
    //                 Ok(packet) => match stack_writer.write(packet.get_bytes()).await {
    //                     Ok(_) => (),
    //                     Err(e) => {
    //                         warn!("write pkt to stack failed: {}", e);
    //                         return;
    //                     }
    //                 },
    //                 Err(err) => {
    //                     warn!("read tun failed {:?}", err);
    //                     return;
    //                 }
    //             }
    //         }
    //     });
    //
    //     info!("packet inbound started");
    //     futures::future::select(t2s, s2t).await;
    //     info!("packet inbound exited");
    // }))
}