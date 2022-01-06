pub mod inbound;

pub trait Sink: AsyncRead + AsyncWrite  {}

mod fd_sink;

use tokio::io::{AsyncRead, AsyncWrite};
pub use fd_sink::*;

mod udp_sink;
pub use udp_sink::*;
