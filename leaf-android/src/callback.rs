use std::fmt::{Debug, Formatter};
use jni::JNIEnv;
use jni::objects::{GlobalRef, JObject};
use leaf::callback::Callback;

pub struct JniCallback<'a> {
    env: JNIEnv<'a>,
    obj: JObject<'a>,
}

impl<'a> JniCallback<'a> {
    pub fn new(env: JNIEnv<'a>, obj: JObject<'a>) -> JniCallback<'a> {
        return JniCallback {
            env,
            obj,
        };
    }
}

impl<'a> Debug for JniCallback<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

unsafe impl<'a> Send for JniCallback<'a> {}

unsafe impl<'a> Sync for JniCallback<'a> {}

impl<'a> Callback for JniCallback<'a> {
    fn report_traffic(self: &Self, tx_rate: f32, rx_rate: f32, tx_total: u64, rx_total: u64) {
        self.env.call_method(self.obj, "reportTraffic", "(V)FFJJ", &[
            tx_rate.into(),
            rx_rate.into(),
            (tx_total as i64).into(),
            (rx_total as i64).into(),
        ])
            .unwrap();
    }
}