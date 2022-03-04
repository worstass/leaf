use std::fmt::{Debug, Formatter};
use leaf::callback::Callback as Inner;
use std::option::Option;
use std::os::raw::{c_float, c_ulonglong};

// #[repr(C)]
// pub struct Callback {
//     pub report_traffic: Option<extern "C" fn(
//         tx_rate: c_float,
//         rx_rate: c_float,
//         tx_total: c_ulonglong,
//         rx_total: c_ulonglong,
//     ) -> ()>,
// }

pub(crate) struct GrpcCallback {
    // inner: *const Callback,
}

impl GrpcCallback {
    pub fn new() -> GrpcCallback {
        return GrpcCallback {
        };
    }
}

impl Debug for GrpcCallback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

unsafe impl Send for GrpcCallback {}

unsafe impl Sync for GrpcCallback {}

impl Inner for GrpcCallback {
    fn report_traffic(self: &Self, tx_rate: f32, rx_rate: f32, rx_total: u64, tx_total: u64) {
        // unsafe {
        //     let f = (* self.inner).report_traffic.unwrap();
        //     f(tx_rate as c_float, rx_rate as c_float, rx_total as c_ulonglong, tx_total as c_ulonglong);
        // }
    }
}