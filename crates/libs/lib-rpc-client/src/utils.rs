use anyhow::{bail, Result};
use bytes::Bytes;
use reqwest::blocking::Response;
use serde::{Deserialize, Serialize};

use crate::{error::ClientError, grpc::proxy::Proxy};
use lib_utils::error::BiliError;

// ==================== impl ResponseExt ====================

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct BiliResponse<T: Serialize = serde_json::Value> {
    pub code: i64,
    pub message: String,
    pub ttl: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(bound(deserialize = "T: Deserialize<'de> + Clone"))]
    pub data: Option<T>,
}

impl<T: Serialize> Default for BiliResponse<T> {
    fn default() -> Self {
        Self {
            code: 5500900,
            message: String::new(),
            ttl: 1,
            data: None,
        }
    }
}

pub struct ResponseExt<H, D> {
    /// Original request
    o_req: reqwest::Request,
    /// Original proxy
    o_proxy: Option<reqwest::Proxy>,
    /// Original response headers, when `T` is actually `reqwest::Response`,
    /// this field will be `()` since `reqwest::Response` will not be consumed
    /// actively.
    resp_headers: H,
    /// Response `data`
    resp_data: D,
}

/// Raw response from upstream with original response.
pub type RawResponseExt = ResponseExt<(), reqwest::Response>;
/// Consumed response from upstream.
pub type ConsumedResponseExt<D = serde_json::Value> = ResponseExt<http_02::HeaderMap, Option<D>>;

impl<H, T> ResponseExt<H, T> {
    pub fn new(
        o_req: reqwest::Request,
        o_proxy: Option<reqwest::Proxy>,
        resp_headers: H,
        resp_data: T,
    ) -> Self {
        Self {
            o_req,
            o_proxy,
            resp_headers,
            resp_data,
        }
    }

    pub fn o_req(&self) -> &reqwest::Request {
        &self.o_req
    }

    pub fn o_proxy(&self) -> Option<&reqwest::Proxy> {
        self.o_proxy.as_ref()
    }
}

impl RawResponseExt {
    fn check_response_status(&self) -> Result<()> {
        let status = self.resp_data.status();
        if status.is_client_error() || status.is_server_error() {
            tracing::error!(
                "Invalid response with HTTP StatusCode [{}]",
                status.as_u16()
            );
            tracing::trace!(
                "Invalid response with headers [{:?}]",
                &self.resp_data.headers()
            );
            bail!(ClientError::HttpStatus(status.as_u16()))
        }
        Ok(())
    }

    /// Into original response
    pub fn into_inner(self) -> reqwest::Response {
        self.resp_data
    }

    /// Consumes reqwest::Response and return `ConsumedResponseExt` with headers
    /// and simple text.
    pub async fn text(self) -> Result<ConsumedResponseExt<String>> {
        self.check_response_status()?;
        let mut response = self.resp_data;
        let resp_headers = std::mem::take(response.headers_mut());
        let resp_data = response.text().await.map_err(|e| ClientError::from(e))?;
        Ok(ConsumedResponseExt {
            o_req: self.o_req,
            o_proxy: self.o_proxy,
            resp_headers,
            resp_data: Some(resp_data),
        })
    }

    /// Consumes reqwest::Response and return `ConsumedResponseExt` with headers
    /// and simple Bytes.
    pub async fn bytes(self) -> Result<ConsumedResponseExt<Bytes>> {
        self.check_response_status()?;
        let mut response = self.resp_data;
        let resp_headers = std::mem::take(response.headers_mut());
        let resp_data = response.bytes().await.map_err(|e| ClientError::from(e))?;
        Ok(ConsumedResponseExt {
            o_req: self.o_req,
            o_proxy: self.o_proxy,
            resp_headers,
            resp_data: Some(resp_data),
        })
    }

    /// Consumes reqwest::Response and return `ConsumedResponseExt` with headers
    /// and deserialized JSON data.
    ///
    /// Generic `D` defaults to be `serde_json::Value`, or you can specify one
    pub async fn json<D>(self) -> Result<ConsumedResponseExt<D>>
    where
        D: for<'de> serde::Deserialize<'de>,
    {
        self.check_response_status()?;
        let mut response = self.resp_data;
        let resp_headers = std::mem::take(response.headers_mut());
        let resp_data = response
            .json::<D>()
            .await
            .map_err(|e| ClientError::from(e))?;
        Ok(ConsumedResponseExt {
            o_req: self.o_req,
            o_proxy: self.o_proxy,
            resp_headers,
            resp_data: Some(resp_data),
        })
    }

    /// Consumes reqwest::Response and return `ConsumedResponseExt` with headers
    /// and deserialized JSON data, Bilibili's API specified
    ///
    /// Generic `D` defaults to be `serde_json::Value`, or you can specify one
    pub async fn bili_json(self) -> Result<ConsumedResponseExt<serde_json::Value>> {
        let ResponseExt {
            o_req,
            o_proxy,
            resp_headers,
            resp_data,
        } = self.json::<BiliResponse>().await?;
        let resp_data = Ok(resp_data).and_then(|data| {
            if let Some(data) = data {
                // Check if is standard BiliResponse
                if data.code == 5500900 {
                    tracing::warn!(
                        "BiliError: Not standard response, try parse whole response like data field"
                    );
                    bail!(BiliError::ReqRiskControl)
                }

                // Check BiliError from ECode and EMessage
                if let Ok(error) = BiliError::try_from((data.code, data.message.as_str())) {
                    tracing::error!(
                        "BiliError: {:?}, with original resp: {:?}",
                        error,
                        serde_json::to_string(&data)
                    );
                    bail!(error)
                }

                let data = if let Some(data) = data.data {
                    // Check possible dirty data
                    if let Some(v_voucher) = data.get("v_voucher") {
                        // Only with v_voucher field in data
                        if data.as_object().unwrap().len() == 1 {
                            tracing::error!(
                                "BiliError: Req risk controlled1 v_voucher [{:?}]",
                                v_voucher
                            );
                            bail!(BiliError::ReqRiskControl)
                        } else {
                            // Not know exactly if is risk controlled
                            // Log and continue
                            tracing::warn!(
                                "BiliError: May be req risk controlled, resp: {:?}",
                                &data
                            );
                        }
                    }
                    Some(data)
                } else {
                    None
                };

                Ok(data)
            } else {
                Ok(None)
            }
        })?;

        Ok(ConsumedResponseExt {
            o_req,
            o_proxy,
            resp_headers,
            resp_data,
        })
    }
}

// ======== impl for ConsumedResponseExt ========

impl<D> ConsumedResponseExt<D> {
    /// Get response header
    pub fn get_header(&self, key: &str) -> Option<&reqwest::header::HeaderValue> {
        self.resp_headers.get(key)
    }

    /// Get response headers ref
    pub fn headers(&self) -> &reqwest::header::HeaderMap {
        &self.resp_headers
    }

    /// Get response headers formatted string
    ///
    /// For debug only
    pub fn headers_str(&self) -> String {
        format!("{:?}", &self.resp_headers)
    }

    /// Get data field ref
    pub fn data(&self) -> Option<&D> {
        self.resp_data.as_ref()
    }

    /// Get data field mut ref
    pub fn data_mut(&mut self) -> Option<&mut D> {
        self.resp_data.as_mut()
    }

    /// Get data field
    pub fn into_data(self) -> Option<D> {
        self.resp_data
    }

    /// Break [`ConsumedResponseExt`] into parts
    pub fn into_parts(
        self,
    ) -> (
        reqwest::Request,
        Option<reqwest::Proxy>,
        http_02::HeaderMap, // compatibility for reqwest with dep:http v0.2
        Option<D>,
    ) {
        (self.o_req, self.o_proxy, self.resp_headers, self.resp_data)
    }
}

// xor-shift
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn fast_random() -> u64 {
    use std::cell::Cell;
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::num::Wrapping;

    thread_local! {
        static RNG: Cell<Wrapping<u64>> = Cell::new(Wrapping(seed()));
    }

    fn seed() -> u64 {
        let seed = RandomState::new();

        let mut out = 0;
        let mut cnt = 0;
        while out == 0 {
            cnt += 1;
            let mut hasher = seed.build_hasher();
            hasher.write_usize(cnt);
            out = hasher.finish();
        }
        out
    }

    RNG.with(|rng| {
        let mut n = rng.get();
        debug_assert_ne!(n.0, 0);
        n ^= n >> 12;
        n ^= n << 25;
        n ^= n >> 27;
        rng.set(n);
        n.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    })
}