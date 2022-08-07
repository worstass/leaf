use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use futures::ready;
use tokio::io::{AsyncBufRead, AsyncWrite};

pub struct Stats {
    pub uplink_counter: Arc<Counter>,
    pub downlink_counter: Arc<Counter>,
}

impl Stats {
    pub fn new() -> Self {
        return Self {
            uplink_counter: Arc::new(Counter::new()),
            downlink_counter: Arc::new(Counter::new()),
        };
    }
}

pub struct Counter {
    pub amt: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        return Self {
            amt: AtomicU64::new(0),
        };
    }
}

struct CountedCopyBuf<'a, R: ?Sized, W: ?Sized> {
    reader: &'a mut R,
    writer: &'a mut W,
    amt: u64,
    cnt: Option<Arc<Counter>>,
}

impl<R, W> Future for CountedCopyBuf<'_, R, W>
    where
        R: AsyncBufRead + Unpin + ?Sized,
        W: AsyncWrite + Unpin + ?Sized,
{
    type Output = io::Result<u64>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let me = &mut *self;
            let buffer = ready!(Pin::new(&mut *me.reader).poll_fill_buf(cx))?;
            if buffer.is_empty() {
                ready!(Pin::new(&mut self.writer).poll_flush(cx))?;
                return Poll::Ready(Ok(self.amt));
            }

            let i = ready!(Pin::new(&mut *me.writer).poll_write(cx, buffer))?;
            if i == 0 {
                return Poll::Ready(Err(std::io::ErrorKind::WriteZero.into()));
            }
            self.amt += i as u64;
            if let Some(s) = self.cnt.clone() {
                (*s).amt.fetch_add(i as u64, Ordering::SeqCst);
            }
            Pin::new(&mut *self.reader).consume(i);
        }
    }
}

pub async fn copy_buf<'a, R, W>(reader: &'a mut R, writer: &'a mut W, cnt: Option<Arc<Counter>>) -> io::Result<u64>
    where
        R: AsyncBufRead + Unpin + ?Sized,
        W: AsyncWrite + Unpin + ?Sized,
{
    CountedCopyBuf {
        reader,
        writer,
        amt: 0,
        cnt,
    }.await
}