use std::collections::VecDeque;
use std::net::SocketAddr;
use std::os::raw;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use futures::Stream;
use smoltcp::iface::Interface;
use crate::{Endpoint, TcpStream};

pub(crate) struct TcpListenerInner {
    queue: VecDeque<TcpStream>,
    waker: Option<Waker>,
    iface: Arc<Mutex<Interface<'static, Endpoint>>>,
}

impl TcpListenerInner {
    pub fn new(iface: Arc<Mutex<Interface<'static, Endpoint>>>, ) -> Self {
        Self {
            queue: VecDeque::new(),
            waker: None,
            iface,
        }
    }
    pub fn add(self: &mut Self, stream: TcpStream) {
        self.queue.push_back(stream);
        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }
}

impl Stream for TcpListenerInner {
    type Item = (TcpStream, SocketAddr, SocketAddr);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(stream) = self.queue.pop_front() {
            let local_addr = stream.local_addr().to_owned();
            let remote_addr = stream.remote_addr().to_owned();
            return Poll::Ready(Some((stream, local_addr, remote_addr)));
        } else {
            self.waker.replace(cx.waker().clone());
            Poll::Pending
        }
    }
}
pub struct TcpListener {
    inner: Arc<Mutex<TcpListenerInner>>,
}

impl TcpListener {
    pub(crate) fn new( inner: Arc<Mutex<TcpListenerInner>>, iface: Arc<Mutex<Interface<'static, Endpoint>>>,
    ) -> Self {
        Self {
            inner,
        }
    }

    pub fn add(self: &mut Self, stream: TcpStream) {
        // self.queue.push_back(stream);
        // if let Some(waker) = self.waker.as_ref() {
        //     waker.wake_by_ref();
        // }
    }
}

impl Stream for TcpListener {
    type Item = (TcpStream, SocketAddr, SocketAddr);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut inner = self.inner.lock().unwrap();
        Pin::new(&mut *inner).poll_next(cx)
    }
}
