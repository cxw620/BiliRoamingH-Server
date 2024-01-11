use axum::{response::IntoResponse, routing::MethodRouter, Json};
use serde_json::json;

use super::{HandlerFuture, ServiceResult, ServiceResultIntoResponse};
use lib_utils::{error::ServerErrorExt, url::QueryMap};

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
                // For historical reason, app only accept non standard response with only `data`
                Self::PgcPlayerApi => res.map_or_else(
                    |e| ServerErrorExt::from(e).into_response(),
                    |v| Json(v).into_response(),
                ),
                _ => res.into_response(),
            }
        })
    }
}

impl PlayurlHandler {
    pub async fn get_playurl(&self, req: axum::extract::Request) -> ServiceResult<serde_json::Value> {
        let query_map = QueryMap::try_from_req(&req)?;
        // TODO implement get playurl
        Ok(json!({
            "is_test": true,
            "path": req.uri().path(),
            "query": query_map.into_inner(),
        }))
    }
}
