use std::pin::Pin;
use tonic::{transport::Server, Request, Response, Status};
use tonic::codegen::futures_core::Stream;
use tonic_web;

pub mod echo {
    tonic::include_proto!("grpc.gateway.testing");
}

use echo::echo_service_server::{EchoService, EchoServiceServer};
use echo::{
    EchoRequest, EchoResponse,
    ServerStreamingEchoRequest,
    ServerStreamingEchoResponse,
};

#[derive(Debug, Default)]
pub struct MyEchoService {}

#[tonic::async_trait]
impl EchoService for MyEchoService {
    type ServerStreamingEchoStream = Pin<Box<dyn Stream<Item = Result<ServerStreamingEchoResponse, Status>> + Send + 'static>>;

    async fn echo(
        &self,
        request: Request<EchoRequest>,
    ) -> Result<Response<EchoResponse>, Status> {
        println!("Got a request: {:?}", request);

        let res = EchoResponse {
            message: format!("Hello {}!", request.into_inner().message).into(),
        };

        Ok(Response::new(res))
    }

    async fn server_streaming_echo(
        &self,
        request: Request<ServerStreamingEchoRequest>,
    ) -> Result<Response<Self::ServerStreamingEchoStream>, Status> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let echo = MyEchoService::default();
    let echo = EchoServiceServer::new(echo);
    let echo = tonic_web::config()
        .allow_all_origins()
        // .allow_origins(vec!["127.0.0.1"])
        .enable(echo);

    println!("EchoServer listening on {}", addr);

    Server::builder()
        .accept_http1(true)
        .add_service(echo)
        .serve(addr)
        .await?;
    Ok(())
}