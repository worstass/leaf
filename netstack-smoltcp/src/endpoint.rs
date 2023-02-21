use std::collections::VecDeque;
use smoltcp::phy;
use smoltcp::phy::{ChecksumCapabilities, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;

pub(crate) struct RxToken {
    buffer: Vec<u8>,
}

impl phy::RxToken for RxToken {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
        where
            F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        f.call_once((&mut self.buffer[..], ))
    }
}

pub(crate) struct TxToken {
// struct TxToken<'a> {
    // queue: &'a mut VecDeque<Vec<u8>>,
}

impl<'a> phy::TxToken for TxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
        where
            F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buf: Vec<u8> = Vec::new();
        buf.resize(len, 0);
        let result = f.call_once((&mut buf[..], ));
        // self.queue.push_back(buf);
        result
    }
}

#[derive(Debug)]
pub(crate) struct Endpoint {
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

    pub fn recv_packet(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inqueue.push_back(Vec::from(buf));
        Ok(())
    }

    pub fn extract_packet(&mut self, mut buf: &mut [u8]) -> Option<()> {
        match self.outbuf.pop() {
            None => None,
            Some(v) => {
                std::io::copy(&mut v.as_slice(), &mut buf);
                Some(())
            }
        }
    }
}

impl<'d> Device<'d> for Endpoint {
    type RxToken = RxToken;
    type TxToken = TxToken;

    fn receive(&mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        self.inqueue.pop_front().map(move |buffer| {
            let rx = RxToken { buffer };
            let tx = TxToken {
                // queue: &mut self.inqueue,
            };
            (rx, tx)
        })
    }

    fn transmit(&mut self) -> Option<Self::TxToken> {
        Some(TxToken {
            // queue: &mut self.inqueue,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut cap = DeviceCapabilities::default();
        cap.medium = Medium::Ip;
        cap.max_transmission_unit = 1500;
        cap.checksum = ChecksumCapabilities::default();
        cap
    }
}