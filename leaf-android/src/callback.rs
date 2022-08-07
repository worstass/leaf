use std::fmt::{Debug, Formatter};
use jni::JNIEnv;
use jni::objects::{GlobalRef, JObject, JValue};
use jni::sys::{jfloat, jlong};
use leaf::callback::Callback;
use crate::jint;

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
        f.debug_struct("JniCallback")
            // .field("env", &self.env.ok())
            // .field("peer_addr", &self.peer_addr().ok())
            .finish()
    }
}

unsafe impl<'a> Send for JniCallback<'a> {}

unsafe impl<'a> Sync for JniCallback<'a> {}

impl<'a> Callback for JniCallback<'a> {
    fn report_traffic(self: &Self, tx_rate: f32, rx_rate: f32, tx_total: i64, rx_total: i64) {
        println!("tx_rate: {}", tx_rate);
        let a:jfloat = tx_rate as jfloat;
        let b:jfloat = rx_rate as jfloat;
        let c:jlong = tx_total as jlong; //.into();// tx_total as jlong;
        let d:jlong = rx_total as jlong;
        match self.env.call_method(self.obj, "reportTraffic", "(FFJJ)V", &[
            JValue::from(0.0),
            JValue::from(0.0),
            JValue::from(0i64),
            JValue::from(0i64),
            // a.into(),b.into(),(1 as i64).into(),(1 as i64).into(),
            // 0.0.into(), 1.1.into(), 1.into(), 2.into(),
        ]) {
            Ok(_) => {}
            Err(e) => {
                println!("{:?}", e)
            }
        };


        // self.env.call_method(self.obj, "reportTraffic", "(FFJJ)V", &[
        //     tx_rate.into(),
        //     rx_rate.into(),
        //     (tx_total as i64).into(),
        //     (rx_total as i64).into(),
        // ]).unwrap();
    }

    fn report_state(self: &Self, state: i32) {
        let s: jint = state as jint;
        self.env.call_method(self.obj, "reportState", "(I)V", &[s.into(), ]).unwrap();
    }
}