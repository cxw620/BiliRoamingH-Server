use anyhow::Result;
use serde_json::json;

use crate::{axum_response, axum_route, generate_router, HandlerFuture};
use lib_utils::url::QueryMap;

generate_router!(
    PlayurlRouter,
    ("/pgc/player/api/playurl", GET, PlayurlHandler::PgcPlayerApi),
    ("/pgc/player/web/playurl", GET, PlayurlHandler::PgcPlayerWeb)
);

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
    pub async fn get_playurl(&self, req: axum::extract::Request) -> Result<serde_json::Value> {
        let query_map = QueryMap::try_from_req(&req)?;
        // TODO implement get playurl
        Ok(json!({
            "is_test": true,
            "path": req.uri().path(),
            "query": query_map.into_inner(),
        }))
    }
}
