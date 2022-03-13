use std::{
    fmt::Debug,
    collections::HashMap,
    sync::Arc,
    sync::Mutex
};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use lazy_static::lazy_static;
use rand::Rng;

use crate::{Runner, RuntimeId};

lazy_static! {
    pub static ref CALLBACKS: Mutex<Arc<HashMap<RuntimeId, Box<dyn Callback>>>> =
        Mutex::new(Arc::new(HashMap::new()));
}

pub trait Callback : Debug + Send + Sync {
   fn report_traffic(self: &Self, tx_rate: f32, rx_rate: f32, rx_total: u64, tx_total: u64);
}

pub fn fake_callback_runner(cb: Box<dyn Callback>) -> Runner {
    Box::pin( async move {
        let mut up_total = 0;
        let mut down_total =0;
        let mut end = std::time::Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let begin = std::time::Instant::now();
            let elapsed = begin - end;
            let mut rng = rand::thread_rng();
            let up = rng.gen_range(0..16384);
            let down = rng.gen_range(0..16384);
            up_total += up;
            down_total += down;
            let up_rate = (up as f32) / (elapsed.as_secs() as f32);
            let down_rate = (down as f32) / (elapsed.as_secs() as f32);
            cb.report_traffic(up_rate, down_rate, up_total, down_total);
            end = begin;
        }
    })
}
