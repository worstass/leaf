use std::pin::Pin;
use tonic::{transport::Server, Request, Response, Status};
use tonic::codegen::futures_core;

pub mod proxie {
    tonic::include_proto!("proxie");
}

use proxie::proxie_server::{Proxie, ProxieServer};
use proxie::{
    QueryRequest, QueryResponse,
    StartRequest, StartResponse,
    StopRequest, StopResponse,
    ListenVpnUpdateRequest,
    VpnUpdate,
};
use crate::futures_core::Stream;

#[derive(Debug, Default)]
pub struct ProxieService {}

#[tonic::async_trait]
impl Proxie for ProxieService {
    type ListenVpnUpdateStream = Pin<Box<dyn Stream<Item = Result<VpnUpdate, Status>> + Send + 'static>>;

    async fn start(
        &self,
        request: Request<StartRequest>,
    ) -> Result<Response<StartResponse>, Status> {
        println!("Got a request: {:?}", request);

        let options = request.into_inner().options;
        println!("{}", options["config"]);

        Ok(Response::new(StartResponse {}))
    }

    async fn stop(
        &self,
        request: Request<StopRequest>,
    ) -> Result<Response<StopResponse>, Status> {
        println!("Got a request: {:?}", request);

        let res = StopResponse {
            // message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(res))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        println!("Got a request: {:?}", request);

        let res = QueryResponse {
            // message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(res))
    }

    async fn listen_vpn_update(
        &self,
        request: Request<ListenVpnUpdateRequest>,
    ) -> Result<Response<Self::ListenVpnUpdateStream>, Status> {
        todo!()
    }
}

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