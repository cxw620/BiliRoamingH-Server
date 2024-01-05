use anyhow::anyhow;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;

use lib_utils::error::{ServerError, ServerErrorExt, TError};

pub type ServiceResult<T> = Result<T, anyhow::Error>;

pub trait ServiceResultIntoResponse<T> {
    fn into_response(self) -> axum::response::Response;
}

impl<T: Serialize> ServiceResultIntoResponse<T> for ServiceResult<T> {
    fn into_response(self) -> axum::response::Response {
        match self {
            Ok(data) => GeneralResponse {
                code: 0,
                message: "".to_string(),
                ttl: 1,
                data: Some(data),
            }
            .into_response(),
            Err(err) => ServerErrorExt::from(err).into_response(),
        }
    }
}

pub type HandlerFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = axum::response::Response> + Send>>;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralResponse<T: Serialize> {
    pub code: i64,
    pub message: String,
    pub ttl: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(bound(deserialize = "T: Deserialize<'de> + Clone"))]
    pub data: Option<T>,
}

impl<T: Serialize> Default for GeneralResponse<T> {
    fn default() -> Self {
        Self {
            code: -500,
            message: "Default JSON Response".to_string(),
            ttl: 1,
            data: None,
        }
    }
}

impl<T: Serialize> IntoResponse for GeneralResponse<T> {
    fn into_response(self) -> axum::response::Response {
        let mut res = serde_json::to_string(&self).unwrap().into_response();
        res.headers_mut().insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/json"),
        );
        res
    }
}

impl<T: Serialize> From<T> for GeneralResponse<T> {
    fn from(value: T) -> Self {
        Self {
            data: Some(value),
            ..Default::default()
        }
    }
}

// impl<T: Serialize> GeneralResponse<T> {
//     pub fn into_inner(self) -> Result<T> {
//         match self.data {
//             Some(data) => Ok(data),
//             None => Err(GeneralError {
//                 code: self.code,
//                 message: self.message,
//             }),
//         }
//     }
// }

#[derive(Clone)]
pub struct DefaultHandler;

impl<T, S> axum::handler::Handler<T, S> for DefaultHandler {
    type Future = HandlerFuture;

    fn call(self, req: axum::extract::Request, _state: S) -> Self::Future {
        let req_uri = req.uri();
        let data = json!({
            "host": req_uri.host(),
            "path": req_uri.path(),
            "query": req_uri.query(),
        });
        let err = ServerError::ServerInternalNotImpl;
        Box::pin(async move {
            GeneralResponse {
                code: err.e_code(),
                message: err.e_message().to_string(),
                ttl: 1,
                data: Some(data),
            }
            .into_response()
        })
    }
}

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
        let e = match req.uri().path() {
            "/ok_empty" => Ok("ok_empty"),
            "/fatal" => Err(anyhow!(ServerError::ServerFatal)),
            "/services_deprecated" => Err(anyhow!(ServerErrorExt::Any { source: anyhow!(ServerError::ServicesDeprecated) })),
            "/any" => Err(anyhow!(ServerErrorExt::Any { source: anyhow!("anyhow error") })),
            "/custom" => Err(anyhow!(ServerErrorExt::Custom { code: 5_500_000, message: "custom error".to_string() })),
            _ => {
                println!("req.uri().path(): {}", req.uri().path());
                Err(anyhow::anyhow!(ServerError::ServerInternalNotImpl))
            },
        };
        Box::pin(async move { e.into_response() })
    }
}
