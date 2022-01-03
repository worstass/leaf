use std::{io, pin::Pin, sync::Arc};

use futures::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::Mutex as TokioMutex;

use crate::app::dispatcher::Dispatcher;
use crate::app::fake_dns::FakeDns;
use crate::app::nat_manager::NatManager;

use super::stack_impl::NetStackImpl;

pub struct Stack(Box<NetStackImpl>);

impl Stack {
    pub fn new(
        inbound_tag: String,
        dispatcher: Arc<Dispatcher>,
        nat_manager: Arc<NatManager>,
        fakedns: Arc<TokioMutex<FakeDns>>,
    ) -> Self {
        Stack(NetStackImpl::new(
            inbound_tag,
            dispatcher,
            nat_manager,
            fakedns,
        ))
    }
}

impl AsyncRead for Stack {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        AsyncRead::poll_read(Pin::new(&mut self.0), cx, buf)
    }
}

impl AsyncWrite for Stack {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        AsyncWrite::poll_write(Pin::new(&mut self.0), cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        AsyncWrite::poll_flush(Pin::new(&mut self.0), cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        AsyncWrite::poll_shutdown(Pin::new(&mut self.0), cx)
    }
}


