pub mod playurl;

use anyhow::anyhow;
use axum::response::IntoResponse;
use bytes::{BufMut, BytesMut};
use http::{header, HeaderValue};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use lib_utils::error::{ServerError, ServerErrorExt, TError};

pub type ServiceResult<T> = Result<T, anyhow::Error>;

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
            code: 0,
            message: String::new(),
            ttl: 1,
            data: None,
        }
    }
}

impl<T: Serialize> IntoResponse for GeneralResponse<T> {
    fn into_response(self) -> axum::response::Response {
        self.into_response(false)
    }
}

impl<T: Serialize> GeneralResponse<T> {
    /// Create a new [GeneralResponse] with data.
    pub fn new(data: T) -> Self {
        Self {
            data: Some(data),
            ..Default::default()
        }
    }

    /// Create a new [GeneralResponse] from [ServiceResult<T>]
    pub fn new_from_result(service_result: ServiceResult<T>) -> Self {
        match service_result {
            Ok(data) => Self::new(data),
            Err(err) => {
                let err = ServerErrorExt::from(err);
                Self {
                    code: err.e_code(),
                    message: err.e_message().to_string(),
                    ..Default::default()
                }
            }
        }
    }

    /// Customly implement [IntoResponse] for [GeneralResponse<T>].
    ///
    /// For historical reason, sometimes non standard response with only `data` is required.
    pub fn into_response(self, data_only: bool) -> axum::response::Response {
        let mut buf = BytesMut::with_capacity(128).writer();
        if data_only && self.code == 0 {
            serde_json::to_writer(&mut buf, &self.data)
        } else {
            serde_json::to_writer(&mut buf, &self)
        }
        .map_or_else(
            |e| {
                error!("serde_json::to_writer error: {}", e);
                ServerError::Serialization.into_response()
            },
            |_| {
                (
                    [(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    )],
                    buf.into_inner().freeze(),
                )
                    .into_response()
            },
        )
    }
}

#[macro_export]
macro_rules! axum_response {
    ($result:expr) => {
        GeneralResponse::new_from_result($result).into_response(false)
    };
    ($result:expr, $data_only:expr) => {
        GeneralResponse::new_from_result($result).into_response($data_only)
    };
}

#[derive(Clone)]
pub struct DefaultHandler;

impl<T, S> axum::handler::Handler<T, S> for DefaultHandler {
    type Future = HandlerFuture;

    fn call(self, req: axum::extract::Request, _state: S) -> Self::Future {
        Box::pin(async move {
            let req_uri = req.uri();
            warn!(
                "Detect unknown path [{}] with query [{:?}].",
                req_uri.path(),
                req_uri.query()
            );
            ServerError::ServicesUnsupported.into_response()
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
                    error!("req.uri().path(): {}", req.uri().path());
                    Err(anyhow!(ServerError::ServerInternalNotImpl))
                }
            };
            GeneralResponse::new_from_result(data).into_response(false)
        })
    }
}
