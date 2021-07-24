use std::io;

use async_trait::async_trait;

use crate::{
    proxy::{
        OutboundConnect, OutboundDatagram, OutboundTransport, UdpOutboundHandler, DatagramTransportType,
    },
    session::Session,
};

pub struct Handler;

#[async_trait]
impl UdpOutboundHandler for Handler {
    fn connect_addr(&self) -> Option<OutboundConnect> {
        None
    }

    fn transport_type(&self) -> DatagramTransportType {
        DatagramTransportType::Datagram
    }

    async fn handle<'a>(
        &'a self,
        _sess: &'a Session,
        _transport: Option<OutboundTransport>,
    ) -> io::Result<Box<dyn OutboundDatagram>> {
        Err(io::Error::new(io::ErrorKind::Other, "dropped"))
    }
}
