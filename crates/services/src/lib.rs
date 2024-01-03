use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;

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

use lib_utils::error::{ServerError, TError};

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
