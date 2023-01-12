use std::collections::VecDeque;
use std::net::SocketAddr;
use std::os::raw;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use futures::Stream;
use crate::TcpStream;

pub struct TcpListener {
    pub waker: Option<Waker>,
    pub queue: VecDeque<TcpStream>,
}

impl TcpListener {
    pub(crate) fn new() -> Self {
        let listener = TcpListener {
            // tpcb: tpcb as usize,
            waker: None,
            queue: VecDeque::new(),
        };
        listener
    }
}

impl Stream for TcpListener {
    type Item = (TcpStream, SocketAddr, SocketAddr);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
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
