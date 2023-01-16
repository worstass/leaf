use std::collections::VecDeque;
use std::net::SocketAddr;
use std::os::raw;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use futures::Stream;
use crate::TcpStream;

pub(crate) struct TcpListenerInner<'a> {
    queue: VecDeque<TcpStream<'a>>,
    waker: Option<Waker>,
}

impl<'a> TcpListenerInner<'a> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            waker: None,
        }
    }
    pub fn add(self: &mut Self, stream: TcpStream<'a>) {
        self.queue.push_back(stream);
        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }
}

pub struct TcpListener<'a> {
    inner: Arc<Mutex<TcpListenerInner<'a>>>,
}

impl<'a> TcpListener<'a> {
    pub(crate) fn new(inner: Arc<Mutex<TcpListenerInner<'a>>>) -> Self {
        TcpListener { inner }
    }
}

impl<'a> Stream for TcpListener<'a> {
    type Item = (TcpStream<'a>, SocketAddr, SocketAddr);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(stream) = inner.queue.pop_front() {
            let local_addr = stream.local_addr().to_owned();
            let remote_addr = stream.remote_addr().to_owned();
            return Poll::Ready(Some((stream, local_addr, remote_addr)));
        } else {
            inner.waker.replace(cx.waker().clone());
            Poll::Pending
        }
    }
}
