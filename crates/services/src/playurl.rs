use axum::routing::MethodRouter;
use serde_json::json;

use super::{axum_response, GeneralResponse, HandlerFuture, ServiceResult};
use lib_utils::url::QueryMap;

pub struct PlayurlRouter;

impl PlayurlRouter {
    pub fn new() -> axum::Router {
        axum::Router::new()
            .route(
                "/pgc/player/api/playurl",
                MethodRouter::new().get::<PlayurlHandler, ()>(PlayurlHandler::PgcPlayerApi),
            )
            .route(
                "/pgc/player/web/playurl",
                MethodRouter::new().get::<PlayurlHandler, ()>(PlayurlHandler::PgcPlayerWeb),
            )
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum PlayurlHandler {
    /// Path: /pgc/player/web/playurl
    PgcPlayerWeb,
    /// Path: /pgc/player/api/playurl
    PgcPlayerApi,
    /// Path: general
    General,
}

impl<T, S> axum::handler::Handler<T, S> for PlayurlHandler
where
    S: Send + Sync + 'static,
{
    type Future = HandlerFuture;

    fn call(self, req: axum::extract::Request, _state: S) -> Self::Future {
        Box::pin(async move {
            let res = self.get_playurl(req).await;
            match self {
                Self::PgcPlayerApi => axum_response!(res, true),
                _ => axum_response!(res),
            }
        })
    }
}

impl PlayurlHandler {
    pub async fn get_playurl(
        &self,
        req: axum::extract::Request,
    ) -> ServiceResult<serde_json::Value> {
        let query_map = QueryMap::try_from_req(&req)?;
        // TODO implement get playurl
        Ok(json!({
            "is_test": true,
            "path": req.uri().path(),
            "query": query_map.into_inner(),
        }))
    }
}

use lib_bilibili::bapis::app::playerunite::v1::{
    player_server::Player, PlayViewUniteReply, PlayViewUniteReq,
};

#[derive(Debug, Default)]
pub struct GrpcServerPlayerUniteV1;

#[tonic::async_trait]
impl Player for GrpcServerPlayerUniteV1 {
    async fn play_view_unite(
        &self,
        request: tonic::Request<PlayViewUniteReq>,
    ) -> std::result::Result<tonic::Response<PlayViewUniteReply>, tonic::Status> {
        todo!()
    }
}

async fn grpc_server_player_unite_v1(
    req: tonic::Request<PlayViewUniteReq>,
) -> anyhow::Result<tonic::Response<PlayViewUniteReply>> {
    let reply = PlayViewUniteReply::default();
    Ok(tonic::Response::new(reply))
}

