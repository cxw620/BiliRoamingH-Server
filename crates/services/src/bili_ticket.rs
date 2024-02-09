use lib_bilibili::bapis::api::ticket::v1::{
    ticket_server::Ticket, GenWebTicketRequest, GenWebTicketResponse, GetTicketRequest,
    GetTicketResponse,
};
use tracing::{error, info};

#[derive(Debug, Default)]
pub struct GrpcServerApiTicketV1;

#[tonic::async_trait]
impl Ticket for GrpcServerApiTicketV1 {
    async fn get_ticket(
        &self,
        request: tonic::Request<GetTicketRequest>,
    ) -> Result<tonic::Response<GetTicketResponse>, tonic::Status> {
        let req = request.into_inner();
        error!(
            "{}",
            String::from_utf8(req.context.get("x-exbadbasket").unwrap().clone()).unwrap()
        );
        Err(tonic::Status::cancelled(""))
    }
    async fn gen_web_ticket(
        &self,
        request: tonic::Request<GenWebTicketRequest>,
    ) -> std::result::Result<tonic::Response<GenWebTicketResponse>, tonic::Status> {
        todo!()
    }
}

// impl GrpcServerApiTicketV1 {
//     pub fn into_router() -> axum::Router {
//         tonic::transport::Server::builder()
//             .add_service(Self::into_service())
//             .into_router()
//     }
// }

#[cfg(test)]
mod test {
    use super::*;
    use lib_bilibili::bapis::api::ticket::v1::ticket_server::TicketServer;
    use tonic::{transport::Server, Request, Response, Status};
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

    #[tokio::test]
    async fn test() -> Result<(), Box<dyn std::error::Error>> {
        tracing_subscriber::registry().with(fmt::layer()).init();

        let addr = "127.0.0.1:50051".parse()?;

        info!("Listening on {}", addr);

        Server::builder()
            .accept_http1(true)
            .add_service(TicketServer::new(GrpcServerApiTicketV1::default()).accept_compressed(tonic::codec::CompressionEncoding::Gzip))
            .serve(addr)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test2() {
        use std::fs;
        let req = fs::read(r#"D:\LocalDevelop\LocalRepo\Project\BiliRoamingH-Server\crates\services\src\GetTicket"#).unwrap();
        let get_tick_req: GetTicketRequest = prost::Message::decode(&req[..]).unwrap();
    }
}
