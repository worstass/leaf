use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use smoltcp::iface::{Interface, SocketHandle};
use smoltcp::socket::{UdpPacketMetadata, UdpSocketBuffer};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::Endpoint;
use crate::utils::*;

type UdpPkt = (Vec<u8>, SocketAddr, SocketAddr);

pub struct UdpSocket {
    waker: Option<Waker>,
    tx: Sender<UdpPkt>,
    rx: Receiver<UdpPkt>,
    iface: Arc<Mutex<Interface<'static, Endpoint>>>,
    handle: SocketHandle,
}


impl UdpSocket {
    pub(crate) fn new(iface: Arc<Mutex<Interface<'static, Endpoint>>>, buffer_size: usize) -> Box<Self> {
        let (tx, rx): (Sender<UdpPkt>, Receiver<UdpPkt>) = channel(buffer_size);

        let iface_clone = iface.clone();
        let mut iface = iface.lock().unwrap();
        let rx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 64]);
        let tx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 128]);
        let sock = smoltcp::socket::UdpSocket::new(rx_buffer, tx_buffer);
        let handle = iface.add_socket(sock);

        Box::new(Self {
            waker: None,
            tx,
            rx,
            iface: iface_clone.clone(),
            handle,
        })
    }

    pub fn split(self: Box<Self>) -> (SendHalf, RecvHalf) {
        (
            SendHalf { handle: self.handle, iface: self.iface.clone() },
            RecvHalf { socket: self }
        )
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {}
}

impl Stream for UdpSocket {
    type Item = UdpPkt;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(pkt)) => Poll::Ready(Some(pkt)),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub struct SendHalf {
    handle: SocketHandle,
    iface: Arc<Mutex<Interface<'static, Endpoint>>>,
    // pub(crate) pcb: usize,
    // pub(crate) socket: Box<UdpSocket>,
}

impl SendHalf {
    pub fn send_to(
        &self,
        data: &[u8],
        src_addr: &SocketAddr,
        dst_addr: &SocketAddr,
    ) -> io::Result<()> {
        let mut iface = self.iface.lock().unwrap();
        let sock = iface.get_socket::<smoltcp::socket::UdpSocket>(self.handle);
        if sock.can_send() {
           return match sock.send_slice(data, (*dst_addr).into()) {
               Ok(_) => Ok(()),
               Err(_) => Err(broken_pipe()),
           }
        }
        Ok(())
    }
}

pub struct RecvHalf {
    pub(crate) socket: Box<UdpSocket>,
}

impl<'a> RecvHalf {
    pub async fn recv_from(&mut self) -> io::Result<UdpPkt> {
        match self.socket.next().await {
            Some(pkt) => Ok(pkt),
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("recv_from udp socket faied: tx closed"),
            )),
        }
    }
}

impl Stream for RecvHalf {
    type Item = UdpPkt;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.socket).poll_next(cx)
    }
}