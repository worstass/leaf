use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use smoltcp::iface::InterfaceBuilder;
use smoltcp::socket::{TcpSocket, TcpSocketBuffer, UdpPacketMetadata, UdpSocket, UdpSocketBuffer};
use smoltcp::wire::{IpAddress, IpCidr};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::Mutex as TokioMutex;

use crate::app::dispatcher::Dispatcher;
use crate::app::fake_dns::FakeDns;
use crate::app::nat_manager::NatManager;
use super::endpoint::Endpoint;

pub struct NetStack {
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
    fakedns: Arc<TokioMutex<FakeDns>>,
}

impl NetStack {
    pub fn new(
        inbound_tag: String,
        dispatcher: Arc<Dispatcher>,
        nat_manager: Arc<NatManager>,
        fakedns: Arc<TokioMutex<FakeDns>>,
    ) -> Self {
        // let endpoint = Endpoint::new();
        // let ip_addrs = [
        //     IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24),
        // ];
        // let mut builder = InterfaceBuilder::new(device, vec![])
        //     .any_ip(true)
        //     .ip_addrs(ip_addrs);
        // let mut iface = builder.finalize();
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
        //
        // });
        NetStack {
            dispatcher,
            nat_manager,
            fakedns,
        }
    }
}

impl Drop for NetStack {
    fn drop(&mut self) {
        todo!()
    }
}

impl AsyncRead for NetStack {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        todo!()
    }
}

impl AsyncWrite for NetStack {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        todo!()
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
