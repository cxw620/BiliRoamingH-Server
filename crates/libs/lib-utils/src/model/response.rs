use anyhow::Result;
use axum::response::IntoResponse;
use bytes::{BufMut, BytesMut};
use http::{header, HeaderValue};
use serde::{Deserialize, Serialize};
use tracing::error;

use std::{collections::HashMap, fmt::Debug as StdDebug};

use crate::error::{ServerError, ServerErrorExt, TError};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralResponse<T: StdDebug + Serialize> {
    code: i64,
    message: String,
    ttl: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(bound(deserialize = "T: Deserialize<'de> + Clone"))]
    data: Option<T>,

    /// Tracing information
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    logger: HashMap<String, String>,
}

impl<T: StdDebug + Serialize> Default for GeneralResponse<T> {
    fn default() -> Self {
        Self {
            code: 0,
            message: "".to_owned(),
            ttl: 1,
            data: None,
            logger: HashMap::with_capacity(4),
        }
    }
}

impl<T: StdDebug + Serialize> IntoResponse for GeneralResponse<T> {
    #[tracing::instrument(level = "debug", name = "GeneralResponse into_response")]
    fn into_response(self) -> axum::response::Response {
        self.into_response(false)
    }
}

impl<T: StdDebug + Serialize> From<Result<T>> for GeneralResponse<T> {
    fn from(service_result: Result<T>) -> Self {
        match service_result {
            Ok(data) => Self::new(data),
            Err(err) => {
                let err = ServerErrorExt::from(err);
                Self::new_error(err.e_code(), err.e_message())
            }
        }
    }
}

impl<T: StdDebug + Serialize> GeneralResponse<T> {
    /// Create a new [GeneralResponse] with data.
    pub fn new(data: T) -> Self {
        Self {
            data: Some(data),
            ..Default::default()
        }
    }

    /// Create a new [GeneralResponse] with error tracing infos.
    pub fn new_error(code: i64, message: impl ToString) -> Self {
        let mut response = Self {
            code,
            message: message.to_string(),
            ..Default::default()
        };

        let context = {
            use tracing_opentelemetry::OpenTelemetrySpanExt;
            // let context = opentelemetry::Context::current();
            // OpenTelemetry Context is propagation inside code is done via tracing crate
            tracing::Span::current().context()
        };

        opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&context, &mut response.logger);
        });

        response
    }

    /// Customly implement [IntoResponse] for [`GeneralResponse`].
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
