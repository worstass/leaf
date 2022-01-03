use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use smoltcp::Result;
use smoltcp::phy::{self, Checksum, ChecksumCapabilities, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;


#[doc(hidden)]
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

#[doc(hidden)]
pub struct TxToken<'a> {
    queue: &'a mut VecDeque<Vec<u8>>,
}

impl<'a> phy::TxToken for TxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
        where
            F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut buffer = Vec::new();
        buffer.resize(len, 0);
        let result = f(&mut buffer);
        self.queue.push_back(buffer);
        result
    }
}

/// A loopback device.
#[derive(Debug)]
pub struct Endpoint {
    queue: VecDeque<Vec<u8>>,
    medium: Medium,
}

#[allow(clippy::new_without_default)]
impl Endpoint {
    /// Creates a loopback device.
    ///
    /// Every packet transmitted through this device will be received through it
    /// in FIFO order.
    pub fn new(/*medium: Medium*/) -> Endpoint {
        Endpoint {
            queue: VecDeque::new(),
            medium: Medium::Ip,
            // medium,
        }
    }
}

impl<'a> Device<'a> for Endpoint {
    type RxToken = RxToken;
    type TxToken = TxToken<'a>;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        self.queue.pop_front().map(move |buffer| {
            let rx = RxToken { buffer };
            let tx = TxToken {
                queue: &mut self.queue,
            };
            (rx, tx)
        })
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken {
            queue: &mut self.queue,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities::default()
        // DeviceCapabilities {
        //     // max_transmission_unit: 65535,
        //     // max_burst_size: None,
        //     // medium: self.medium,
        //     // // ..DeviceCapabilities::default()
        //     // checksum: ChecksumCapabilities::default(),
        //     medium: Medium::Ip,
        //     max_transmission_unit: 0,
        //     max_burst_size: None,
        //     checksum: ChecksumCapabilities::default(),
        // }
    }
}
