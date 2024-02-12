use anyhow::anyhow;

use crate::{model::GeneralResponse, HandlerFuture};
use lib_utils::error::{ServerError, ServerErrorExt};

#[derive(Default)]
pub struct RouterTest;

impl RouterTest {
    pub fn new() -> axum::Router {
        axum::Router::new().fallback::<_, ()>(TestHandler)
    }
}

#[derive(Clone)]
struct TestHandler;

impl<T, S> axum::handler::Handler<T, S> for TestHandler {
    type Future = HandlerFuture;

    fn call(self, req: axum::extract::Request, _state: S) -> Self::Future {
        Box::pin(async move {
            let data = match req.uri().path() {
                "/ok_empty" => Ok("ok_empty"),
                "/fatal" => Err(anyhow!(ServerError::ServerFatal)),
                "/services_deprecated" => Err(anyhow!(ServerErrorExt::Any {
                    source: anyhow!(ServerError::ServicesDeprecated)
                })),
                "/any" => Err(anyhow!(ServerErrorExt::Any {
                    source: anyhow!("anyhow error")
                })),
                "/custom" => Err(anyhow!(ServerErrorExt::Custom {
                    code: 5_500_000,
                    message: "custom error".to_string()
                })),
                _ => {
                    tracing::error!("req.uri().path(): {}", req.uri().path());
                    Err(anyhow!(ServerError::ServerInternalNotImpl))
                }
            };
            GeneralResponse::new_from_result(data).into_response(false)
        })
    }
}
