use std::{
    io,
    net::SocketAddr,
    os::raw,
    pin::Pin,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Once,
    },
    time,
};
use log::*;
use futures::task::{Context, Poll};
use smoltcp::iface::InterfaceBuilder;
use smoltcp::phy::{Medium, RawSocket};
use tokio::{
    self,
    io::{AsyncRead, AsyncWrite, ReadBuf},
};
use tokio::sync::Mutex as TokioMutex;

use crate::{Dispatcher, NatManager};
use crate::app::fake_dns::FakeDns;

use smoltcp::Result;
use smoltcp::socket::{TcpSocket, UdpSocket};

pub struct NetStackImpl {

}

impl NetStackImpl {
    pub fn new(
        inbound_tag: String,
        dispatcher: Arc<Dispatcher>,
        nat_manager: Arc<NatManager>,
        fakedns: Arc<TokioMutex<FakeDns>>,
    ) -> Box<Self> {
        // let device = RawSocket::new("tun1", Medium::Ip).unwrap();
        //
        // let mut builder = InterfaceBuilder::new(device, vec![])
        //     .any_ip(true)
        //     .ip_addrs(ip_addrs);
        // // builder = builder
        // //     .hardware_addr(ieee802154_addr.into())
        // //     .neighbor_cache(neighbor_cache);
        // let mut iface = builder.finalize();
        //
        // let socket = iface.get_socket::<TcpSocket>(tcp_handle);
        // if socket.is_active() && !tcp_active {
        //     debug!("connected");
        // } else if !socket.is_active() && tcp_active {
        //     debug!("disconnected");
        // }
        // // socket.listen()
        // let socket1 = iface.get_socket::<UdpSocket>(tcp_handle);

        let stack = Box::new(NetStackImpl {
            // lwip_lock: Arc::new(AtomicMutex::new()),
            // waker: None,
            // tx,
            // rx,
            // dispatcher,
            // nat_manager,
            // fakedns,
        });
        stack
    }
}

impl Drop for NetStackImpl {
    fn drop(&mut self) {
        log::trace!("drop netstack");
        // unsafe {
        //     let _g = self.lwip_lock.lock();
        //     OUTPUT_CB_PTR = 0x0;
        // };
    }
}

impl AsyncRead for NetStackImpl {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for NetStackImpl {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(0))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}