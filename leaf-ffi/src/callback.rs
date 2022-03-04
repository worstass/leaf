use std::fmt::{Debug, Formatter};
use leaf::callback::Callback as Inner;
use std::option::Option;
use std::os::raw::{c_float, c_ulonglong};

#[repr(C)]
pub struct Callback {
    pub report_traffic: Option<extern "C" fn(
        tx_rate: c_float,
        rx_rate: c_float,
        tx_total: c_ulonglong,
        rx_total: c_ulonglong,
    ) -> ()>,
}

pub(crate) struct FfiCallback {
    inner: *const Callback,
}

impl FfiCallback {
    pub fn new(inner: *const Callback) -> FfiCallback {
        return FfiCallback {
            inner
        };
    }
}

impl Debug for FfiCallback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

unsafe impl Send for FfiCallback {}

unsafe impl Sync for FfiCallback {}

impl Inner for FfiCallback {
    fn report_traffic(self: &Self, tx_rate: f32, rx_rate: f32, rx_total: u64, tx_total: u64) {
        unsafe {
            let f = (* self.inner).report_traffic.unwrap();
            f(tx_rate as c_float, rx_rate as c_float, rx_total as c_ulonglong, tx_total as c_ulonglong);
        }
    }
}