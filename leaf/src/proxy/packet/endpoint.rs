use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use smoltcp::time::Instant;
use smoltcp::phy;
use smoltcp::phy::{Medium, Device, DeviceCapabilities};
use smoltcp::Result;

pub struct RxToken {
    buffer: Vec<u8>,
}

impl phy::RxToken for RxToken {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> Result<R>
        where
            F: FnOnce(&mut [u8]) -> Result<R>,
    {
        f(&mut self.buffer)
    }
}

pub struct TxToken {
    // queue: VecDeque<Vec<u8>>,
}

impl<'a> phy::TxToken for TxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
        where
            F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut buffer = Vec::new();
        buffer.resize(len, 0);
        let result = f(&mut buffer);
        // self.queue.push_back(buffer);
        result
    }
}

#[derive(Debug)]
pub struct Endpoint {
    inqueue: VecDeque<Vec<u8>>,
    outbuf: Vec<Vec<u8>>,
}

#[allow(clippy::new_without_default)]
impl Endpoint {
    pub fn new() -> Endpoint {
        Endpoint {
            inqueue: VecDeque::new(),
            outbuf: Vec::new(),
        }
    }

    pub fn inject_packet(self: &mut Self, buf: &[u8]) -> std::io::Result<()> {
        self.inqueue.push_back(Vec::from(buf));
        Ok(())
    }

    pub fn extract_packet(self: &mut Self, mut buf: &mut [u8]) -> Option<()> {
        match self.outbuf.pop() {
            None => None,
            Some(v) => {
                std::io::copy( &mut v.as_slice(), &mut buf);
                Some(())
            }
        }
    }
}

impl<'a> Device<'a> for Endpoint {
    type RxToken = RxToken;
    type TxToken = TxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        self.inqueue.pop_front().map(move |buffer| {
            let rx = RxToken { buffer };
            let tx = TxToken {
                // queue: &mut self.inqueue,
            };
            (rx, tx)
        })
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken {
            // queue: &mut self.inqueue,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities::default()
    }
}
