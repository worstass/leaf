use std::pin::Pin;
use tonic::{transport::Server, Request, Response, Status};
use tonic::codegen::futures_core;
use tonic::codegen::futures_core::Stream;
use leaf::RuntimeId;

// pub mod proxie {
//     tonic::include_proto!("proxie");
// }

tonic::include_proto!("proxie");

pub use proxie_server::{Proxie, ProxieServer};

use crate::GrpcCallback;
// pub use {
//     QueryRequest, QueryResponse,
//     StartRequest, StartResponse,
//     StopRequest, StopResponse,
//     ListenVpnUpdateRequest,
//     VpnUpdate,
// };

// use crate::futures_core::Stream;

const RT_ID: RuntimeId = 0;

#[derive(Debug, Default)]
pub struct ProxieService {}

#[tonic::async_trait]
impl Proxie for ProxieService {
    type ListenVpnUpdateStream = Pin<Box<dyn Stream<Item = Result<VpnUpdate, Status>> + Send + 'static>>;

    async fn start(
        &self,
        request: Request<StartRequest>,
    ) -> Result<Response<StartResponse>, Status> {
        // println!("Got a request: {:?}", request);
        let json = request.into_inner().json;
        println!("{}", json);
        let cfg = "".to_string();

        let opts = leaf::StartOptions {
            config: leaf::Config::Str(cfg),
            #[cfg(feature = "callback")] callback: Box::new( GrpcCallback::new()),
            #[cfg(feature = "auto-reload")]
            auto_reload: false,
            runtime_opt: leaf::RuntimeOption::SingleThread,
        };
        if let Err(e) = leaf::start(RT_ID, opts) {}
        Ok(Response::new(StartResponse {}))
    }

    async fn stop(
        &self,
        request: Request<StopRequest>,
    ) -> Result<Response<StopResponse>, Status> {
        println!("Got a request: {:?}", request);
        if !leaf::shutdown(RT_ID) {}
        Ok(Response::new(StopResponse {}))
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