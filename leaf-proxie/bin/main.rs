use std::pin::Pin;
use std::result::Result;
use std::boxed::Box;
use tonic::transport::Server;

use leaf_proxie::{
    ProxieServer, ProxieService,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8081".parse().unwrap();
    let proxie = ProxieService::default();
    let proxie = ProxieServer::new(proxie);
    let proxie = tonic_web::config()
        .allow_all_origins()
        .enable(proxie);
    println!("Proxie server listening on {}", addr);
    Server::builder()
        .accept_http1(true)
        .add_service(proxie)
        .serve(addr)
        .await?;
    Ok(())
}