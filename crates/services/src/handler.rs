pub mod playurl;
pub mod test;

use anyhow::Result;
use axum::extract::Request as AxumRequest;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response as AxumResponse;
use lib_utils::error::ServerErrorExt;

use std::future::Future;

use crate::{
    intercept::{DefaultInterceptor, InterceptT},
    HandlerFuture,
};
use lib_utils::error::ServerError;

/// A trait for handling requests.
pub trait HandlerT: 'static + Sized + std::fmt::Debug + Clone + Send {
    type Response: IntoResponse;

    /// Call the handler.
    ///
    /// DO NOT use this method directly, use `call_for_response` instead.
    fn call(self, req: AxumRequest) -> impl Future<Output = Result<Self::Response>> + Send;

    /// Call the handler and return the response
    fn call_for_response(self, req: AxumRequest) -> impl Future<Output = AxumResponse> + Send {
        async {
            self.call(req)
                .await
                .map(|resp| resp.into_response())
                .unwrap_or_else(|err| ServerErrorExt::from(err).into_response())
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct DefaultHandler;

impl HandlerT for DefaultHandler {
    type Response = AxumResponse;

    #[tracing::instrument(skip(self), name="DefaultHandler")]
    async fn call(self, req: AxumRequest) -> Result<Self::Response> {
        let req_uri = req.uri();
        let response = match req_uri.path() {
            "/favicon.ico" => StatusCode::NOT_FOUND.into_response(),
            _ => {
                tracing::warn!(
                    "Detect unknown path [{}] with query [{:?}].",
                    req_uri.path(),
                    req_uri.query()
                );
                return Err(ServerError::ServicesUnsupported.into());
            }
        };
        Ok(response)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InterceptHandler<R: InterceptT = DefaultInterceptor, H: HandlerT = DefaultHandler> {
    pub interceptor: Option<R>,
    pub handler: H,
    desc: &'static str
}

impl Default for InterceptHandler {
    fn default() -> Self {
        InterceptHandler {
            interceptor: None,
            handler: DefaultHandler,
            desc: "InterceptHandler for default"
        }
    }
}

impl<T, S, R: InterceptT, H: HandlerT> axum::handler::Handler<T, S> for InterceptHandler<R, H> {
    type Future = HandlerFuture;

    #[tracing::instrument(skip(self, _state), name="InterceptHandler", fields(intercept.desc=self.desc))]
    fn call(self, mut req: axum::extract::Request, _state: S) -> Self::Future {
        Box::pin(async move {
            if let Some(interceptor) = &self.interceptor {
                if let Err(e) = interceptor.intercept_request(&mut req).await {
                    return ServerErrorExt::from(e).into_response();
                }
            }

            let mut response = self.handler.call_for_response(req).await;

            if let Some(interceptor) = &self.interceptor {
                match interceptor.intercept_response(&mut response).await {
                    Ok(Some(new_response)) => {
                        response = new_response;
                    }
                    Err(e) => {
                        return ServerErrorExt::from(e).into_response();
                    }
                    _ => {}
                }
            }

            response
        })
    }
}
