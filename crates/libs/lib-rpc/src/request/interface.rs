use anyhow::Result;
use http_02::{HeaderMap as HttpHeaderMap, Method as HttpMethod};
use url::Url;

use std::{borrow::Cow, fmt::Debug as FmtDebug, future::Future};

use super::client::{
    rest::{ReqBody, RestRequest, RestRequestBuilder},
    utils::RawResponseExt,
};
use crate::utils::{ManagedHeaderMap, Upstream};

/// The trait definition for a RESTful RPC request.
pub trait RpcT<'r> {
    /// Pre-defined request method. Default to `GET`.
    const METHOD: HttpMethod = HttpMethod::GET;
    /// Pre-defined request authority.
    const UPSTREAM: Upstream<'static>;
    /// Pre-defined request path. Default to `""`.
    const PATH: &'static str = "";

    #[tracing::instrument(level = "debug", name = "RpcT.execute_rpc")]
    fn execute_rpc(
        proxy: Option<&'r str>,
        query: Option<Cow<'r, str>>,
        headers: Option<impl Into<HttpHeaderMap> + FmtDebug>,
        body: Option<impl Into<ReqBody> + FmtDebug>,
    ) -> impl Future<Output = Result<RawResponseExt>> + Send {
        GeneralRpc::new((), Self::UPSTREAM)
            .with_proxy(proxy)
            .with_path(Self::PATH)
            .with_query(query)
            .with_headers(headers)
            .with_body(body)
            .execute()
    }
}

/// The trait definition for a RPC Request builder
pub trait RpcBuilderT<'r>: Sized {
    /// Pre-defined request method. Default to `GET`.
    const DEFAULT_HTTP_METHORD: HttpMethod = HttpMethod::GET;
    /// Pre-defined request authority.
    const DEFAULT_UPSTREAM: Upstream<'r>;

    type Request;
    type Response;

    /// Create a new instance of the rpc builder
    fn new(request: Self::Request, upstream: impl Into<Upstream<'r>>) -> Self;

    /// Create a new instance of the rpc builder with default upstream
    #[inline]
    fn new_default_upstream(request: Self::Request) -> Self {
        Self::new(request, Self::DEFAULT_UPSTREAM)
    }

    /// Set the method for the RPC request
    fn with_method(self, _method: HttpMethod) -> Self {
        self
    }

    /// Set the upstream for the RPC request
    fn with_upstream(self, upstream: impl Into<Upstream<'r>>) -> Self;

    /// Set the proxy for the RPC request
    fn with_proxy(self, proxy: Option<&'r str>) -> Self;

    /// Set the path for the RPC request
    fn with_path(self, _path: &'r str) -> Self {
        self
    }

    /// Set the query for the RPC request
    fn with_query(self, _query: Option<Cow<'r, str>>) -> Self {
        self
    }

    /// Set the headers for the RPC request
    fn with_headers(self, headers: Option<impl Into<HttpHeaderMap>>) -> Self;

    /// Set the headers(managed) for the RPC request
    fn with_headers_managed(self, headers: Option<impl Into<ManagedHeaderMap>>) -> Self;

    /// Set the body for the RPC request
    fn with_body(self, _body: Option<impl Into<ReqBody>>) -> Self {
        self
    }

    /// Execute the RPC request
    fn execute(self) -> impl Future<Output = Result<Self::Response>> + Send;
}

pub struct GeneralRpc<'r> {
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

impl<'r> RpcBuilderT<'r> for GeneralRpc<'r> {
    const DEFAULT_UPSTREAM: Upstream<'r> = Upstream::new_custom("http://no.default.upstream.com");

    type Request = ();
    type Response = RawResponseExt;

    #[inline]
    fn new(_request: Self::Request, upstream: impl Into<Upstream<'r>>) -> Self {
        Self {
            method: HttpMethod::GET,
            upstream: upstream.into(),
            path: "",
            query: None,
            inner: RestRequest::builder(),
        }
    }

    #[inline]
    fn with_method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    #[inline]
    fn with_upstream(mut self, upstream: impl Into<Upstream<'r>>) -> Self {
        self.upstream = upstream.into();
        self
    }

    #[inline]
    fn with_proxy(mut self, proxy: Option<&'r str>) -> Self {
        self.inner = self.inner.proxy(proxy);
        self
    }

    #[inline]
    fn with_path(mut self, path: &'r str) -> Self {
        self.path = path;
        self
    }

    #[inline]
    fn with_query(mut self, query: Option<Cow<'r, str>>) -> Self {
        self.query = query;
        self
    }

    #[inline]
    fn with_headers(mut self, headers: Option<impl Into<HttpHeaderMap>>) -> Self {
        let headers = headers.map(|h| h.into());
        self.inner = self.inner.headers(headers);
        self
    }

    #[inline]
    fn with_headers_managed(mut self, headers: Option<impl Into<ManagedHeaderMap>>) -> Self {
        self.inner = self.inner.headers(headers.map(|h| h.into().take_inner()));
        self
    }

    #[inline]
    fn with_body(mut self, body: Option<impl Into<ReqBody>>) -> Self {
        self.inner = self.inner.body(body);
        self
    }

    #[tracing::instrument(level = "debug", name = "GeneralRpc.execute", skip_all, err)]
    async fn execute(self) -> Result<Self::Response> {
        let full_url = self.upstream.url().map(|mut u: Url| {
            u.set_path(self.path);
            u.set_query(self.query.as_deref());
            u
        })?;

        self.inner.build_with(full_url).execute(self.method).await
    }
}
