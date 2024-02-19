use anyhow::Result;
use axum::extract::Request as AxumRequest;

use super::{DefaultInterceptor, HandlerT, InterceptHandler};
use crate::{generate_router, http02_compat, impl_rpc_t};
use lib_utils::model::response::ResponsePassthrough;

generate_router!(
    TestInterceptRouter,
    (
        "/x/web-interface/zone",
        GET,
        InterceptHandler::new(Some(DefaultInterceptor), TestHandler, "Test Intercept")
    )
);

#[derive(Debug, Clone)]
pub struct TestHandler;

impl_rpc_t!(TestHandler, Upstream::API_DEFAULT, "/x/web-interface/zone");

impl HandlerT for TestHandler {
    type Response = ResponsePassthrough;

    #[tracing::instrument(level = "debug", name = "TestHandler.call", skip(self), err)]
    async fn call(self, req: AxumRequest) -> Result<Self::Response> {
        let (parts, body) = req.into_parts();
        // poll data from request Body
        let body = axum::body::to_bytes(body, usize::MAX).await?;

        let headers = parts.headers;

        http02_compat!(headers_http02, headers, http_02);

        let (_, _, resp_headers_http02, data) =
            <Self as RpcT>::execute_rpc(None, None, Some(headers_http02), Some(body))
                .await?
                .bytes()
                .await?
                .into_parts();

        http02_compat!(resp_headers, resp_headers_http02, http);

        Ok(ResponsePassthrough {
            headers: resp_headers,
            body: data.unwrap_or_default(),
        })
    }
}
