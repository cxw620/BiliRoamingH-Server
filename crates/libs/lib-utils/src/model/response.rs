use anyhow::Result;
use axum::response::{IntoResponse, Response as AxumResponse};
use bytes::{BufMut, BytesMut};
use http::{header, HeaderValue};
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, fmt::Debug as StdDebug};

use crate::error::{ServerError, ServerErrorExt, TError};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralResponse<T: StdDebug + Serialize = serde_json::Value> {
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
    fn into_response(self) -> AxumResponse {
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
    #[inline]
    pub fn new(data: T) -> Self {
        Self {
            data: Some(data),
            ..Default::default()
        }
    }

    /// Create a new [GeneralResponse] with or without data.
    #[inline]
    pub fn new_or_empty(data: Option<T>) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

    /// Create a new [GeneralResponse] with error tracing infos.
    #[inline]
    #[tracing::instrument(skip_all)]
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
    #[inline]
    #[tracing::instrument(skip(self))]
    pub fn into_response(self, data_only: bool) -> AxumResponse {
        let mut buf = BytesMut::with_capacity(128).writer();
        if data_only && self.code == 0 {
            serde_json::to_writer(&mut buf, &self.data)
        } else {
            serde_json::to_writer(&mut buf, &self)
        }
        .map_or_else(
            |e| {
                tracing::error!("serde_json::to_writer error: {}", e);
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

/// A wrapper for [GeneralResponse] with headers.
pub struct GeneralResponseExt<T: StdDebug + Serialize = serde_json::Value> {
    inner: GeneralResponse<T>,
    headers: http::HeaderMap,
}

impl<T: StdDebug + Serialize> GeneralResponseExt<T> {
    #[inline]
    pub fn new(inner: T, headers: http::HeaderMap) -> Self {
        Self {
            inner: GeneralResponse::new(inner),
            headers,
        }
    }

    #[inline]
    pub fn new_or_empty(inner: Option<T>, headers: http::HeaderMap) -> Self {
        Self {
            inner: GeneralResponse::new_or_empty(inner),
            headers,
        }
    }

    #[inline]
    pub fn into_response(self, data_only: bool) -> AxumResponse {
        let mut response = self.inner.into_response(data_only);
        *response.headers_mut() = self.headers;
        response
    }
}

impl<T: StdDebug + Serialize> IntoResponse for GeneralResponseExt<T> {
    fn into_response(self) -> AxumResponse {
        self.into_response(false)
    }
}

/// A wrapper for passing through response from upstream.
#[derive(Debug)]
pub struct ResponsePassthrough {
    pub headers: http::HeaderMap,
    pub body: bytes::Bytes,
}

impl IntoResponse for ResponsePassthrough {
    #[tracing::instrument(skip(self))]
    fn into_response(self) -> AxumResponse {
        let mut response = AxumResponse::new(self.body.into());
        *response.headers_mut() = self.headers;
        response
    }
}
