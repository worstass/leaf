use std::{
    fmt::Debug,
    collections::HashMap,
    sync::Arc,
    sync::Mutex,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::time::Duration;

use lazy_static::lazy_static;
use log::*;
use rand::Rng;

use crate::{Runner, RuntimeId};
use crate::app::stats::Stats;

lazy_static! {
    pub static ref CALLBACKS: Mutex<Arc<HashMap<RuntimeId, Box<dyn Callback>>>> =
        Mutex::new(Arc::new(HashMap::new()));
}

pub trait Callback: Debug + Send + Sync {
    fn report_traffic(self: &Self, tx_rate: f32, rx_rate: f32, rx_total: u64, tx_total: u64);
}

#[derive(Debug)]
pub struct ConsoleCallback {}

impl ConsoleCallback {
    pub fn new() -> Self {
        return Self {};
    }
}

impl Callback for ConsoleCallback {
    fn report_traffic(self: &Self, tx_rate: f32, rx_rate: f32, rx_total: u64, tx_total: u64) {
        debug!("traffic: up_rate: {} down_rate {} up_total: {} down_total {}", tx_rate, rx_rate, rx_total, tx_total);
    }
}

pub fn fake_callback_runner(cb: Box<dyn Callback>) -> Runner {
    Box::pin(async move {
        let mut up_total = 0;
        let mut down_total = 0;
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

pub fn stats_callback_runner(cb: Box<dyn Callback>, stats: Arc<Stats>) -> Runner {
    Box::pin(async move {
        let mut last_up_total = 0;
        let mut last_down_total = 0;
        let mut end = std::time::Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let begin = std::time::Instant::now();
            let elapsed = begin - end;
            let mut rng = rand::thread_rng();
            let up_total = (&stats.uplink_counter.clone().amt).load(Ordering::Acquire);
            let down_total = (&stats.downlink_counter.clone().amt).load(Ordering::Acquire);
            let mut up = up_total - last_up_total;
            let mut down = down_total - last_down_total;
            let up_rate = (up as f32) / (elapsed.as_secs() as f32);
            let down_rate = (down as f32) / (elapsed.as_secs() as f32);
            last_up_total = up_total;
            last_down_total = down_total;
            cb.report_traffic(up_rate, down_rate, up_total, down_total);
            end = begin;
        }
    })
}