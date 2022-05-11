use std::{io, pin::Pin};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::ready;
use futures::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A proxy stream writes a header along with the first payload.
pub struct BufHeadProxyStream<T> {
    inner: T,
    head: Option<Bytes>,
    first_payload: BytesMut,
}

impl<T> BufHeadProxyStream<T> {
    pub fn new(inner: T, head: Bytes) -> Self {
        Self {
            inner,
            head: Some(head),
            first_payload: BytesMut::new(),
        }
    }
}

impl<T> AsyncRead for BufHeadProxyStream<T>
    where
        T: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

fn early_eof() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "early eof")
}

impl<T> AsyncWrite for BufHeadProxyStream<T>
    where
        T: AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let me = &mut *self;
        // Combine header and first payload.
        if let Some(head) = me.head.take() {
            me.first_payload.put_slice(&head);
            me.first_payload.put_slice(buf);
        }
        while !me.first_payload.is_empty() {
            let n = ready!(Pin::new(&mut me.inner).poll_write(cx, &me.first_payload))?;
            if n == 0 {
                return Poll::Ready(Err(early_eof()));
            }
            me.first_payload.advance(n);
            if me.first_payload.is_empty() {
                me.first_payload = BytesMut::new(); // shadow to free
                return Poll::Ready(Ok(buf.len()));
            }
        }
        Pin::new(&mut me.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}