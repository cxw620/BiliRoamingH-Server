pub mod playurl_compat;

use anyhow::Result;
use axum::response::IntoResponse;
use bytes::{BufMut, BytesMut};
use http::{header, HeaderValue};
use serde::{Deserialize, Serialize};
use tracing::error;

use lib_utils::error::{ServerError, ServerErrorExt, TError};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralResponse<T: std::fmt::Debug + Serialize> {
    pub code: i64,
    pub message: String,
    pub ttl: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(bound(deserialize = "T: Deserialize<'de> + Clone"))]
    pub data: Option<T>,
}

impl<T: std::fmt::Debug + Serialize> Default for GeneralResponse<T> {
    fn default() -> Self {
        Self {
            code: 0,
            message: String::new(),
            ttl: 1,
            data: None,
        }
    }
}

impl<T: std::fmt::Debug + Serialize> IntoResponse for GeneralResponse<T> {
    #[tracing::instrument(level = "debug", name="GeneralResponse into_response")]
    fn into_response(self) -> axum::response::Response {
        self.into_response(false)
    }
}

impl<T: std::fmt::Debug + Serialize> GeneralResponse<T> {
    /// Create a new [GeneralResponse] with data.
    pub fn new(data: T) -> Self {
        Self {
            data: Some(data),
            ..Default::default()
        }
    }

    /// Create a new [GeneralResponse] from [Result<T>]
    pub fn new_from_result(service_result: Result<T>) -> Self {
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
