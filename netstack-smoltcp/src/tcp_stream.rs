use std::cmp::min;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::raw;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use log::trace;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::io::{AsyncBufReadExt};
use bytes::BytesMut;
use smoltcp::iface::{Interface, SocketHandle};
use smoltcp::socket::TcpSocket;
use crate::Endpoint;
use crate::utils::*;
// fn broken_pipe() -> io::Error {
//     io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe")
// }

pub struct TcpStream {
    handle: SocketHandle,
    // socket: &'static mut TcpSocket<'static>,
    src_addr: SocketAddr,
    dest_addr: SocketAddr,
    // inner: Box<TcpStreamImpl>,
    write_buf: BytesMut,

    iface: Arc<Mutex<Interface<'static, Endpoint>>>,
}

impl TcpStream {
    pub(crate) fn new(
        handle: SocketHandle,
        // socket: TcpSocket,
        iface: Arc<Mutex<Interface<'static, Endpoint>>>,
    ) -> Self {
        let src_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 1111);
        let dest_addr =    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 2222);

        // let mut ifac = iface.lock().unwrap();
        // let sock = ifac.get_socket::<TcpSocket>(handle);
        let stream = TcpStream {
            handle,
            src_addr,
            dest_addr,
            write_buf: BytesMut::new(),
            iface,
            // socket:sock,
        };
        stream
    }

    pub fn local_addr(&self) -> &SocketAddr {
        &self.src_addr
    }

    pub fn remote_addr(&self) -> &SocketAddr {
        &self.dest_addr
    }

}

impl AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        let mut iface = self.iface.lock().unwrap();
        let sock = iface.get_socket::<TcpSocket>(self.handle);
        if sock.can_recv() {
           let data = sock.recv(|data| {
                let mut data = data.to_owned();
                if !data.is_empty() {
                    // debug!(
                    //         "recv data: {:?}",
                    //         str::from_utf8(data.as_ref()).unwrap_or("(invalid utf8)")
                    //     );
                    // data = data.split(|&b| b == b'\n').collect::<Vec<_>>().concat();
                    // data.reverse();
                    // data.extend(b"\n");
                }
                (data.len(), data)
            }).unwrap();
            buf.put_slice(&data[..]);
        }
        Poll::Ready(Ok(()))
        // let me = &mut *self;
        //
        // // handle any previously unsent data
        // if !me.write_buf.is_empty() {
        //     let to_read = min(buf.remaining(), me.write_buf.len());
        //     let piece = me.write_buf.split_to(to_read);
        //     buf.put_slice(&piece[..to_read]);
        //     return Poll::Ready(Ok(()));
        // }
        // match Pin::new(&mut ctx.read_rx).poll_recv(cx) {
        //     Poll::Ready(Some(data)) => {
        //         let to_read = min(buf.remaining(), data.len());
        //         buf.put_slice(&data[..to_read]);
        //         if to_read < data.len() {
        //             me.write_buf.extend_from_slice(&data[to_read..]);
        //         }
        //         // unsafe { tcp_recved(me.pcb as *mut tcp_pcb, data.len() as u16_t) };
        //         Poll::Ready(Ok(()))
        //     }
        //     Poll::Ready(None) => Poll::Ready(Err(broken_pipe())),
        //     Poll::Pending => {
        //         // no more buffered data
        //         if ctx.eof {
        //             Poll::Ready(Ok(())) // eof
        //         } else {
        //             Poll::Pending
        //         }
        //     }
        // }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut iface = self.iface.lock().unwrap();
        let sock = iface.get_socket::<TcpSocket>(self.handle);
        if sock.can_send() {
            return match sock.send_slice(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(_) => Poll::Ready(Err(broken_pipe())),
            }
        }
        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        todo!()
    }
}