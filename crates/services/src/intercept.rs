pub(crate) mod bilibili;

use std::future::Future;

use anyhow::Result;
use axum::extract::Request as AxumRequest;
use axum::response::Response as AxumResponse;

pub trait InterceptT: 'static + std::fmt::Debug + Clone + Send {
    #[tracing::instrument(skip(self))]
    /// Intercept request headers or body, return `Ok(())` to continue
    /// or error stop the request.
    fn intercept_request(
        &self,
        request: &mut AxumRequest,
    ) -> impl Future<Output = Result<()>> + Send {
        async {
            Ok(())
        }
    }

    /// Intercept response headers or bodys, modify original response,
    /// return new [AxumResponse] or error.
    #[tracing::instrument(skip(self))]
    fn intercept_response(
        &self,
        response: &mut AxumResponse,
    ) -> impl Future<Output = Result<Option<AxumResponse>>> + Send {
        async {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DefaultInterceptor;
impl InterceptT for DefaultInterceptor {}
