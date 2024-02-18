use anyhow::Result;
use http_02::{HeaderMap as HttpHeaderMap, Method as HttpMethod};
use url::Url;

use std::{borrow::Cow, future::Future};

use super::client::{
    rest::{ReqBody, RestRequest, RestRequestBuilder},
    utils::RawResponseExt,
};
use crate::utils::Upstream;

/// The trait definition for a RPC request.
pub trait RpcT<'r> {
    /// Pre-defined request method. Default to `GET`.
    const METHOD: HttpMethod = HttpMethod::GET;
    /// Pre-defined request authority.
    const UPSTREAM: Upstream<'static>;
    /// Pre-defined request path. Default to `""`.
    const PATH: &'static str = "";

    fn execute_rpc(
        proxy: Option<&'r str>,
        query: Option<Cow<'r, str>>,
        headers: Option<impl Into<HttpHeaderMap>>,
        body: Option<impl Into<ReqBody>>,
    ) -> impl Future<Output = Result<RawResponseExt>> + Send {
        GeneralRequest::new(Self::METHOD, Self::UPSTREAM, Self::PATH)
            .with_proxy(proxy)
            .with_query(query)
            .with_headers(headers)
            .with_body(body)
            .execute()
    }
}

pub struct GeneralRequest<'r> {
    /// The method of the request. Default to `GET`.
    method: HttpMethod,
    /// The authority of the request.
    upstream: Upstream<'r>,
    /// The path of the request.
    path: &'r str,
    /// The query of the request.
    query: Option<Cow<'r, str>>,
    ///
    inner: RestRequestBuilder<'r>,
}

impl<'r> GeneralRequest<'r> {
    #[inline]
    pub fn new(method: HttpMethod, upstream: impl Into<Upstream<'r>>, path: &'r str) -> Self {
        Self {
            method,
            upstream: upstream.into(),
            path,
            query: None,
            inner: RestRequest::builder(),
        }
    }

    /// Configure proxy for the request
    #[inline]
    pub fn with_proxy(mut self, proxy: Option<&'r str>) -> Self {
        self.inner = self.inner.proxy(proxy);
        self
    }

    /// Configure query for the request
    #[inline]
    pub fn with_query(mut self, query: Option<Cow<'r, str>>) -> Self {
        self.query = query;
        self
    }

    /// Configure headers for the request
    #[inline]
    pub fn with_headers(mut self, headers: Option<impl Into<HttpHeaderMap>>) -> Self {
        self.inner = self.inner.headers(headers);
        self
    }

    /// Configure body for the request
    #[inline]
    pub fn with_body(mut self, body: Option<impl Into<ReqBody>>) -> Self {
        self.inner = self.inner.body(body);
        self
    }

    #[inline]
    pub async fn execute(self) -> Result<RawResponseExt> {
        let full_url = self.upstream.url().map(|mut u: Url| {
            u.set_path(self.path);
            u.set_query(self.query.as_deref());
            u
        })?;

        self.inner.build_with(full_url).execute(self.method).await
    }
}
