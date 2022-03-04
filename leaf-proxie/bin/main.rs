use std::pin::Pin;
use std::result::Result;
use std::boxed::Box;
use std::env;

use tonic::transport::Server;

use leaf_proxie::{
    proxie::{ProxieServer, ProxieService},
    runat::{RunAtServer, RunAtService},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("GRPC_PORT").unwrap_or("8081".to_string());
    let addr = format!("127.0.0.1:{}", port).parse().unwrap();
    let proxie = ProxieService::default();
    let proxie = ProxieServer::new(proxie);
    let proxie = tonic_web::config()
        .allow_all_origins()
        .enable(proxie);
    let runat = RunAtService::default();
    let runat = RunAtServer::new(runat);
    let runat = tonic_web::config()
        .allow_all_origins()
        .enable(runat);
    println!("Proxie server listening on {}", addr);
    Server::builder()
        .accept_http1(true)
        .add_service(proxie)
        .add_service(runat)
        .serve(addr)
        .await?;
    Ok(())
}