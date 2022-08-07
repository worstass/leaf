use std::ffi::c_void;
use std::fmt::{Debug, Formatter};
use std::mem::size_of;
use leaf::callback::Callback as Inner;
use std::option::Option;
use std::os::raw::{c_float, c_longlong};
use std::ptr::null;
use libc::{c_int, free};

#[repr(C)]
pub struct Callback {
    report_traffic: Option<extern "C" fn(
        tx_rate: c_float,
        rx_rate: c_float,
        tx_total: c_longlong,
        rx_total: c_longlong,
    ) -> ()>,
    report_state: Option<extern "C" fn(
        state: c_int,
    ) -> ()>,
}

#[no_mangle]
pub extern "C" fn create_callback(
    report_traffic: Option<extern "C" fn(
        tx_rate: c_float,
        rx_rate: c_float,
        tx_total: c_longlong,
        rx_total: c_longlong,
    ) -> ()>,
    report_state: Option<extern "C" fn(
        state: c_int,
    ) -> ()>,
) -> *mut Callback {
    unsafe {
        let p = libc::malloc(size_of::<Callback>()) as *mut Callback;
        (*p).report_traffic = report_traffic;
        (*p).report_state = report_state;
        p
    }
}

#[no_mangle]
pub extern "C" fn destroy_callback(cb: *mut Callback) {
    unsafe { free(cb as *mut c_void) }
}

#[derive(Debug)]
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

unsafe impl Send for FfiCallback {}

unsafe impl Sync for FfiCallback {}

impl Inner for FfiCallback {
    fn report_traffic(self: &Self, tx_rate: f32, rx_rate: f32, rx_total: i64, tx_total: i64) {
        unsafe {
            let f = (*self.inner).report_traffic.unwrap();
            f(tx_rate as c_float, rx_rate as c_float, rx_total as c_longlong, tx_total as c_longlong);
        }
    }

    fn report_state(self: &Self, state: i32) {
        unsafe {
            let f = (*self.inner).report_state.unwrap();
            f(state as c_int);
        }
    }
}