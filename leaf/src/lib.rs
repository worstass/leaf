use std::collections::HashMap;
use std::io;
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Once;
use std::time::Duration;

use anyhow::anyhow;
use lazy_static::lazy_static;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

#[cfg(feature = "auto-reload")]
use notify::{
    event, Error as NotifyError, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher,
};
use tokio::time::sleep;

use app::{
    dispatcher::Dispatcher, dns_client::DnsClient, inbound::manager::InboundManager,
    nat_manager::NatManager, outbound::manager::OutboundManager, router::Router,
};

#[cfg(feature = "stat")]
use crate::app::{stat_manager::StatManager, SyncStatManager};

#[cfg(feature = "api")]
use crate::app::api::api_server::ApiServer;

pub mod app;
pub mod common;
pub mod config;
pub mod option;
pub mod proxy;
pub mod session;
pub mod util;

#[cfg(any(target_os = "ios", target_os = "macos", target_os = "android"))]
pub mod mobile;

// MARKER BEGIN
// #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
#[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux", target_os = "windows")))]
mod sys;

#[cfg(feature = "callback")] pub mod callback; // MARKER BEGIN - END
#[cfg(feature = "callback")]
use callback::{
    Callback,
    fake_callback_runner, stat_callback_runner,
};
use crate::callback::{ConsoleCallback, STATE_LOCAL_STARTED, STATE_LOCAL_STARTING, STATE_LOCAL_STOPPED, STATE_LOCAL_STOPPING,
                      // stats_callback_runner
};
// MARKER BEGIN - END

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] anyhow::Error),
    #[error("no associated config file")]
    NoConfigFile,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[cfg(feature = "auto-reload")]
    #[error(transparent)]
    Watcher(#[from] NotifyError),
    #[error(transparent)]
    AsyncChannelSend(
        #[from] tokio::sync::mpsc::error::SendError<std::sync::mpsc::SyncSender<Result<(), Error>>>,
    ),
    #[error(transparent)]
    SyncChannelRecv(#[from] std::sync::mpsc::RecvError),
    #[error("runtime manager error")]
    RuntimeManager,
}

pub type Runner = futures::future::BoxFuture<'static, ()>;

pub struct RuntimeManager {
    #[cfg(feature = "auto-reload")]
    rt_id: RuntimeId,
    config_path: Option<String>,
    #[cfg(feature = "auto-reload")]
    auto_reload: bool,
    reload_tx: mpsc::Sender<std::sync::mpsc::SyncSender<Result<(), Error>>>,
    shutdown_tx: mpsc::Sender<()>,
    router: Arc<RwLock<Router>>,
    dns_client: Arc<RwLock<DnsClient>>,
    outbound_manager: Arc<RwLock<OutboundManager>>,
    #[cfg(feature = "stat")]
    stat_manager: SyncStatManager,
    #[cfg(feature = "auto-reload")]
    watcher: Mutex<Option<RecommendedWatcher>>,
}

impl RuntimeManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        #[cfg(feature = "auto-reload")] rt_id: RuntimeId,
        config_path: Option<String>,
        #[cfg(feature = "auto-reload")] auto_reload: bool,
        reload_tx: mpsc::Sender<std::sync::mpsc::SyncSender<Result<(), Error>>>,
        shutdown_tx: mpsc::Sender<()>,
        router: Arc<RwLock<Router>>,
        dns_client: Arc<RwLock<DnsClient>>,
        outbound_manager: Arc<RwLock<OutboundManager>>,
        #[cfg(feature = "stat")] stat_manager: SyncStatManager,
    ) -> Arc<Self> {
        Arc::new(Self {
            #[cfg(feature = "auto-reload")]
            rt_id,
            config_path,
            #[cfg(feature = "auto-reload")]
            auto_reload,
            reload_tx,
            shutdown_tx,
            router,
            dns_client,
            outbound_manager,
            #[cfg(feature = "stat")]
            stat_manager,
            #[cfg(feature = "auto-reload")]
            watcher: Mutex::new(None),
        })
    }

    #[cfg(feature = "stat")]
    pub fn stat_manager(&self) -> SyncStatManager {
        self.stat_manager.clone()
    }

    pub async fn set_outbound_selected(&self, outbound: &str, select: &str) -> Result<(), Error> {
        if let Some(selector) = self.outbound_manager.read().await.get_selector(outbound) {
            selector
                .write()
                .await
                .set_selected(select)
                .map_err(Error::Config)
        } else {
            Err(Error::Config(anyhow!("selector not found")))
        }
    }

    pub async fn get_outbound_selected(&self, outbound: &str) -> Result<String, Error> {
        if let Some(selector) = self.outbound_manager.read().await.get_selector(outbound) {
            return Ok(selector.read().await.get_selected_tag());
        }
        Err(Error::Config(anyhow!("not found")))
    }

    pub async fn get_outbound_selects(&self, outbound: &str) -> Result<Vec<String>, Error> {
        if let Some(selector) = self.outbound_manager.read().await.get_selector(outbound) {
            return Ok(selector.read().await.get_available_tags());
        }
        Err(Error::Config(anyhow!("not found")))
    }

    // This function could block by an in-progress connection dialing.
    //
    // TODO Reload FakeDns. And perhaps the inbounds as long as the listening
    // addresses haven't changed.
    pub async fn reload(&self) -> Result<(), Error> {
        let config_path = if let Some(p) = self.config_path.as_ref() {
            p
        } else {
            return Err(Error::NoConfigFile);
        };
        log::info!("reloading from config file: {}", config_path);
        let mut config = config::from_file(config_path).map_err(Error::Config)?;
        self.router.write().await.reload(&mut config.router)?;
        self.dns_client.write().await.reload(&config.dns)?;
        self.outbound_manager
            .write()
            .await
            .reload(&config.outbounds, self.dns_client.clone())
            .await?;
        log::info!("reloaded from config file: {}", config_path);
        Ok(())
    }

    pub fn blocking_reload(&self) -> Result<(), Error> {
        let tx = self.reload_tx.clone();
        let (res_tx, res_rx) = sync_channel(0);
        if let Err(e) = tx.blocking_send(res_tx) {
            return Err(Error::AsyncChannelSend(e));
        }
        match res_rx.recv() {
            Ok(res) => res,
            Err(e) => Err(Error::SyncChannelRecv(e)),
        }
    }

    pub async fn shutdown(&self) -> bool {
        let tx = self.shutdown_tx.clone();
        if let Err(e) = tx.send(()).await {
            log::warn!("sending shutdown signal failed: {}", e);
            return false;
        }
        true
    }

    pub fn blocking_shutdown(&self) -> bool {
        let tx = self.shutdown_tx.clone();
        if let Err(e) = tx.blocking_send(()) {
            log::warn!("sending shutdown signal failed: {}", e);
            return false;
        }
        true
    }

    #[cfg(feature = "auto-reload")]
    pub(crate) fn new_watcher(&self) -> Result<(), Error> {
        let config_path = if let Some(p) = self.config_path.as_ref() {
            p
        } else {
            return Err(Error::NoConfigFile);
        };
        if self.auto_reload {
            log::trace!("starting new watcher for config file: {}", config_path);
            let rt_id = self.rt_id;
            let mut watcher: RecommendedWatcher =
                notify::recommended_watcher(move |res: NotifyResult<event::Event>| {
                    match res {
                        // FIXME Not sure what are the most appropriate events to
                        // filter on different platforms.
                        Ok(ev) => {
                            match ev.kind {
                                #[cfg(any(target_os = "macos", target_os = "ios"))]
                                event::EventKind::Modify(event::ModifyKind::Data(
                                    event::DataChange::Content,
                                )) => {
                                    log::info!("config file event matched: {:?}", ev);
                                    if let Err(e) = reload(rt_id) {
                                        log::warn!("reload config file failed: {}", e);
                                    }
                                }
                                #[cfg(any(target_os = "linux", target_os = "android"))]
                                event::EventKind::Access(event::AccessKind::Close(
                                    event::AccessMode::Write,
                                ))
                                | event::EventKind::Remove(event::RemoveKind::File) => {
                                    log::info!("config file event matched: {:?}", ev);
                                    if let Err(e) = reload(rt_id) {
                                        log::warn!("reload config file failed: {}", e);
                                    }
                                }
                                #[cfg(target_os = "windows")]
                                event::EventKind::Modify(event::ModifyKind::Data(
                                    event::DataChange::Any,
                                )) => {
                                    log::info!("config file event matched: {:?}", ev);
                                    if let Err(e) = reload(rt_id) {
                                        log::warn!("reload config file failed: {}", e);
                                    }
                                }
                                _ => {
                                    log::trace!("skip config file event: {:?}", ev);
                                }
                            }
                            // The config file could somehow be removed and re-created
                            // by an editor, in that case create a new watcher to watch
                            // the new file.
                            if let event::EventKind::Remove(event::RemoveKind::File) = ev.kind {
                                if let Ok(g) = RUNTIME_MANAGER.lock() {
                                    if let Some(m) = g.get(&rt_id) {
                                        let _ = m.new_watcher();
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("config file watch error: {:?}", e);
                        }
                    }
                })
                .map_err(Error::Watcher)?;
            watcher
                .watch(
                    std::path::Path::new(&config_path),
                    RecursiveMode::NonRecursive,
                )
                .map_err(Error::Watcher)?;
            log::info!("watching changes of file: {}", config_path);
            self.watcher.lock().unwrap().replace(watcher);
        }
        Ok(())
    }
}

pub type RuntimeId = u16;

lazy_static! {
    pub static ref RUNTIME_MANAGER: Mutex<HashMap<RuntimeId, Arc<RuntimeManager>>> =
        Mutex::new(HashMap::new());
}

pub fn reload(key: RuntimeId) -> Result<(), Error> {
    if let Ok(g) = RUNTIME_MANAGER.lock() {
        if let Some(m) = g.get(&key) {
            return m.blocking_reload();
        }
    }
    Err(Error::RuntimeManager)
}

pub fn shutdown(key: RuntimeId) -> bool {
    if let Ok(g) = RUNTIME_MANAGER.lock() {
        if let Some(m) = g.get(&key) {
            return m.blocking_shutdown();
        }
    }
    false
}

pub fn is_running(key: RuntimeId) -> bool {
    RUNTIME_MANAGER.lock().unwrap().contains_key(&key)
}

pub fn test_config(config_path: &str) -> Result<(), Error> {
    config::from_file(config_path)
        .map(|_| ())
        .map_err(Error::Config)
}

fn new_runtime(opt: &RuntimeOption) -> Result<tokio::runtime::Runtime, Error> {
    match opt {
        RuntimeOption::SingleThread => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(Error::Io),
        RuntimeOption::MultiThreadAuto(stack_size) => tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(*stack_size)
            .enable_all()
            .build()
            .map_err(Error::Io),
        RuntimeOption::MultiThread(worker_threads, stack_size) => {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(*worker_threads)
                .thread_stack_size(*stack_size)
                .enable_all()
                .build()
                .map_err(Error::Io)
        }
    }
}

#[derive(Debug)]
pub enum RuntimeOption {
    // Single-threaded runtime.
    SingleThread,
    // Multi-threaded runtime with thread stack size.
    MultiThreadAuto(usize),
    // Multi-threaded runtime with the number of worker threads and thread stack size.
    MultiThread(usize, usize),
}

#[derive(Debug)]
pub enum Config {
    File(String),
    Str(String),
    Internal(config::Config),
}

#[derive(Debug)]
pub struct StartOptions {
    // The path of the config.
    pub config: Config,
    #[cfg(feature = "callback")] pub callback: Option<Box<dyn Callback>>, // MARKER BEGIN - END
    // Enable auto reload, take effect only when "auto-reload" feature is enabled.
    #[cfg(feature = "auto-reload")]
    pub auto_reload: bool,
    // Tokio runtime options.
    pub runtime_opt: RuntimeOption,
}

pub fn start(rt_id: RuntimeId, opts: StartOptions) -> Result<(), Error> {
    println!("start with options:\n{:#?}", opts);
    // MARKER BEGIN
    #[cfg(feature = "callback")]
    let cb = match opts.callback {
        None => {
            let _cb: Box<dyn Callback> = Box::new(ConsoleCallback::new());
            Some(Arc::new(_cb))
        },
        Some(_cb) => Some(Arc::new(_cb)),
    };
    #[cfg(feature = "callback")]
    if let Some(ref _cb) = cb {
        _cb.clone().report_state(STATE_LOCAL_STARTING)
    }
    // MARKER END
    let (reload_tx, mut reload_rx) = mpsc::channel(1);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    let config_path = match opts.config {
        Config::File(ref p) => Some(p.to_owned()),
        _ => None,
    };

    let mut config = match opts.config {
        Config::File(p) => config::from_file(&p).map_err(Error::Config)?,
        Config::Str(s) => config::from_string(&s).map_err(Error::Config)?,
        Config::Internal(c) => c,
    };

    // FIXME Unfortunately fern does not allow re-initializing the logger,
    // should consider another logging lib if the situation doesn't change.
    let log = config
        .log
        .as_ref()
        .ok_or_else(|| Error::Config(anyhow!("empty log setting")))?;
    static ONCE: Once = Once::new();
    ONCE.call_once(move || {
        app::logger::setup_logger(log).expect("setup logger failed");
    });

    let rt = new_runtime(&opts.runtime_opt)?;
    let _g = rt.enter();

    let mut tasks: Vec<Runner> = Vec::new();
    let mut runners = Vec::new();

    let dns_client = Arc::new(RwLock::new(
        DnsClient::new(&config.dns).map_err(Error::Config)?,
    ));
    let outbound_manager = Arc::new(RwLock::new(
        OutboundManager::new(&config.outbounds, dns_client.clone()).map_err(Error::Config)?,
    ));
    let router = Arc::new(RwLock::new(Router::new(
        &mut config.router,
        dns_client.clone(),
    )));
    // MARKER BEGIN
    #[cfg(feature = "stat")]
    let stats = Arc::new(crate::app::stat::Stats::new());
    // MARKER END
    #[cfg(feature = "stat")]
    let stat_manager = Arc::new(RwLock::new(StatManager::new()));
    #[cfg(feature = "stat")]
    runners.push(StatManager::cleanup_task(stat_manager.clone()));
    let dispatcher = Arc::new(Dispatcher::new(
        outbound_manager.clone(),
        router.clone(),
        dns_client.clone(),
        #[cfg(feature = "stat")] stats.clone(), // MARKER BEGIN -  END
        #[cfg(feature = "stat")]
        stat_manager.clone(),
    ));

    let dispatcher_weak = Arc::downgrade(&dispatcher);
    let dns_client_cloned = dns_client.clone();
    rt.block_on(async move {
        dns_client_cloned
            .write()
            .await
            .replace_dispatcher(dispatcher_weak);
    });

    let nat_manager = Arc::new(NatManager::new(dispatcher.clone()));
    let inbound_manager =
        InboundManager::new(&config.inbounds, dispatcher, nat_manager).map_err(Error::Config)?;
    let mut inbound_net_runners = inbound_manager
        .get_network_runners()
        .map_err(Error::Config)?;
    runners.append(&mut inbound_net_runners);

    // MARKER BEGIN
    // #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    #[cfg(all(any(feature = "inbound-tun", feature = "inbound-packet"), any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let net_info = if inbound_manager.has_tun_listener() || inbound_manager.has_packet_listener()  /*&& inbound_manager.tun_auto()*/ {
        sys::get_net_info()
    } else {
        sys::NetInfo::default()
    };

    // MARKER BEGIN
    // #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    #[cfg(all(any(feature = "inbound-tun", feature = "inbound-packet"), any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        if let sys::NetInfo {
            default_interface: Some(iface),
            ..
        } = &net_info
        {
            let binds = if let Ok(v) = std::env::var("OUTBOUND_INTERFACE") {
                format!("{},{}", v, iface)
            } else {
                iface.clone()
            };
            std::env::set_var("OUTBOUND_INTERFACE", binds);
        }
    }

    #[cfg(all(
        feature = "inbound-tun",
        any(
            // target_os = "ios", // MARKER BEGIN - END
            target_os = "android",
            target_os = "macos", // MARKER BEGIN - END
            target_os = "linux",
            target_os = "windows", // MARKER BEGIN - END
        )
    ))]
    if let Ok(r) = inbound_manager.get_tun_runner() {
        runners.push(r);
    }

    // MARKER BEGIN
    // #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    // sys::post_tun_creation_setup(&net_info);
    #[cfg(all(feature = "inbound-tun", any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    if inbound_manager.tun_auto() { sys::post_tun_creation_setup(&net_info); }

    // MARKER BEGIN
    #[cfg(feature = "inbound-packet")]
    if let Ok(r) = inbound_manager.get_packet_runner() {
        runners.push(r);
    }

    #[cfg(feature = "callback")]
    if let Some(ref _cb) = cb {
        // runners.push(fake_callback_runner(_cb.clone()));
        runners.push(stat_callback_runner(_cb.clone(), stats.clone()));

    }
    // MARKER END

    let runtime_manager = RuntimeManager::new(
        #[cfg(feature = "auto-reload")]
        rt_id,
        config_path,
        #[cfg(feature = "auto-reload")]
        opts.auto_reload,
        reload_tx,
        shutdown_tx,
        router,
        dns_client,
        outbound_manager,
        #[cfg(feature = "stat")]
        stat_manager,
    );

    // Monitor config file changes.
    #[cfg(feature = "auto-reload")]
    {
        if let Err(e) = runtime_manager.new_watcher() {
            log::warn!("start config file watcher failed: {}", e);
        }
    }

    #[cfg(feature = "api")]
    {
        use std::net::SocketAddr;
        let listen_addr = if !(&*option::API_LISTEN).is_empty() {
            Some(
                (&*option::API_LISTEN)
                    .parse::<SocketAddr>()
                    .map_err(|e| Error::Config(anyhow!("parse SocketAddr failed: {}", e)))?,
            )
        } else {
            None
        };
        if let Some(listen_addr) = listen_addr {
            let api_server = ApiServer::new(runtime_manager.clone());
            runners.push(api_server.serve(listen_addr));
        }
    }

    drop(config); // explicitly free the memory

    // Monitor reload signal.
    let rm = runtime_manager.clone();
    tasks.push(Box::pin(async move {
        loop {
            if let Some(res_tx) = reload_rx.recv().await {
                let res = rm.reload().await;
                if let Err(e) = res_tx.send(res) {
                    log::warn!("sending reload result failed: {}", e);
                }
            } else {
                log::warn!("receiving none reload signal");
            }
        }
    }));

    // The main task joining all runners.
    tasks.push(Box::pin(async move {
        futures::future::join_all(runners).await;
    }));

    // Monitor shutdown signal.
    tasks.push(Box::pin(async move {
        let _ = shutdown_rx.recv().await;
    }));

    // Monitor ctrl-c exit signal.
    #[cfg(feature = "ctrlc")]
    tasks.push(Box::pin(async move {
        let _ = tokio::signal::ctrl_c().await;
    }));

    RUNTIME_MANAGER
        .lock()
        .map_err(|_| Error::RuntimeManager)?
        .insert(rt_id, runtime_manager);

    log::trace!("added runtime {}", &rt_id);

    // MARKER BEGIN
    #[cfg(feature = "callback")]
    if let Some(ref _cb) = cb {
        _cb.clone().report_state(STATE_LOCAL_STARTED)
    }
    // MARKER END
    rt.block_on(futures::future::select_all(tasks));

    // MARKER BEGIN
    #[cfg(feature = "callback")]
    if let Some(ref _cb) = cb {
        _cb.clone().report_state(STATE_LOCAL_STOPPING)
    }
    // MARKER END

    // MARKER BEGIN
    // #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    // sys::post_tun_completion_setup(&net_info);
    #[cfg(all(feature = "inbound-tun", any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    if inbound_manager.tun_auto() { sys::post_tun_completion_setup(&net_info); }

    drop(inbound_manager);

    RUNTIME_MANAGER
        .lock()
        .map_err(|_| Error::RuntimeManager)?
        .remove(&rt_id);

    rt.shutdown_background();

    log::trace!("removed runtime {}", &rt_id);

    // MARKER BEGIN
    #[cfg(feature = "callback")]
    if let Some(ref _cb) = cb {
        _cb.clone().report_state(STATE_LOCAL_STOPPED)
    }
    // MARKER END

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_restart() {
        let conf = r#"
[General]
loglevel = trace
dns-server = 1.1.1.1
socks-interface = 127.0.0.1
socks-port = 1080
# tun = auto

[Proxy]
Direct = direct
"#;

        for _i in 1..3 {
            thread::spawn(move || {
                let opts = StartOptions {
                    config: Config::Str(conf.to_string()),
                    #[cfg(feature = "callback")] callback: None, // MARKER BEGIN - END
                    #[cfg(feature = "auto-reload")]
                    auto_reload: false,
                    runtime_opt: RuntimeOption::SingleThread,
                };
                start(0, opts).unwrap();
            });
            thread::sleep(std::time::Duration::from_secs(2));
            assert!(shutdown(0));
            loop {
                thread::sleep(std::time::Duration::from_secs(1));
                if !is_running(0) {
                    break;
                }
            }
        }
    }
}
