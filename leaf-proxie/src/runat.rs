use tonic::{Request, Response, Status};

// pub mod runat {
// }
tonic::include_proto!("runat");

pub use run_at_server::{RunAtServer, RunAt};

// pub use {
//     PingRequest, PingResponse,
// };

#[derive(Debug, Default)]
pub struct RunAtService {}

#[tonic::async_trait]
impl RunAt for RunAtService {
    async fn ping(
        &self,
        request: Request<PingRequest>,
    ) -> Result<Response<PingResponse>, Status> {
        todo!()
    }
}
