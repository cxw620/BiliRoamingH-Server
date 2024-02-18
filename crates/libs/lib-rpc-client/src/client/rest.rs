use anyhow::{anyhow, Result};
use dashmap::DashMap;
use http_02::{HeaderMap as HttpHeaderMap, Method as HttpMethod};
use reqwest::{Client, Proxy};
use url::Url;

use std::{sync::OnceLock, time::Duration};

use lib_utils::headers::ManagedHeaderMap;

use crate::{
    utils::{RawResponseExt, ResponseExt},
    CrateError,
};

// Re-export reqwest::Body
pub use reqwest::Body as ReqBody;

/// Clients with or without proxy
static CLIENTS: OnceLock<DashMap<&'static str, reqwest::Client>> = OnceLock::new();

/// Init Clients with given proxies url.
///
/// Return error if CLIENTS is already inited.
#[tracing::instrument]
pub fn init_reqwest_clients(proxies: Vec<&'static str>) -> Result<()> {
    let map = dashmap::DashMap::with_capacity(16);

    // Default client without proxy
    map.insert("default", gen_client(None)?);

    for p in proxies {
        let rp = Proxy::all(p).map_err(|e| anyhow!(CrateError::from(e)))?;
        map.insert(p, gen_client(Some(rp))?);
    }

    CLIENTS.set(map).map_err(|_| {
        tracing::error!("CLIENTS should be initialized only once");
        anyhow!("CLIENTS should be initialized only once")
    })
}

/// Generate reqwest::Client with given proxy
#[tracing::instrument]
fn gen_client(proxy: Option<reqwest::Proxy>) -> Result<reqwest::Client> {
    let mut builder = Client::builder()
        .use_rustls_tls()
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .tcp_keepalive(Some(Duration::from_secs(3600)))
        .tcp_nodelay(true)
        .pool_idle_timeout(Duration::from_secs(3600))
        // ! Should set UA separately
        // .user_agent(user_agent)
        .http2_keep_alive_interval(Some(Duration::from_secs(18)))
        .http2_keep_alive_while_idle(true)
        .http2_keep_alive_timeout(Duration::from_secs(16))
        .http2_initial_connection_window_size(Some((1 << 28) - 1))
        .http2_initial_stream_window_size(Some((1 << 28) - 1))
        // ! Only accept invalid certs when test
        .danger_accept_invalid_certs(cfg!(test))
        // .danger_accept_invalid_hostnames(cfg!(test)) // rustls not with this
        .connection_verbose(cfg!(test));
    if let Some(proxy) = proxy {
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(|e| anyhow!(CrateError::from(e)))
}

/// Get reqwest::Client from CLIENTS cache or new one with given proxy
#[tracing::instrument]
fn get_client(proxy: Option<&str>) -> Result<reqwest::Client> {
    let clients = CLIENTS.get_or_init(|| {
        tracing::warn!("CLIENTS should be initialized before get_client!!!");
        let map = dashmap::DashMap::with_capacity(16);
        map.insert("default", gen_client(None).unwrap());
        map
    });

    debug_assert!(clients.get("default").is_some());

    let client = clients
        .get(proxy.unwrap_or_else(|| {
            tracing::trace!("proxy is None, use default client");
            "default"
        }))
        .map(|c| c.clone());

    if let Some(client) = client {
        tracing::trace!("Got reqwest::Client from cache");
        Ok(client)
    } else {
        tracing::warn!("Unknown given proxy, Box::leak may cause memory leak");

        let proxy_str = proxy.unwrap();

        let rp = Proxy::all(proxy_str).map_err(|e| anyhow!(CrateError::from(e)))?;

        let client = gen_client(Some(rp))?;
        clients.insert(Box::leak(Box::new(proxy_str.to_string())), client.clone());

        tracing::trace!("Got new reqwest::Client from given proxy [{:?}]", proxy);
        Ok(client)
    }
}

/// [`RestRequest`] with ideal method, url, headers and body.
///
/// **Recommended** Use [`RestRequestBuilder`] to build [`RestRequest`]
#[derive(Debug)]
pub struct RestRequest<'c> {
    pub proxy: Option<&'c str>,
    pub url: Url,
    pub headers: Option<HttpHeaderMap>,
    pub body: Option<reqwest::Body>,
}

impl<'c> RestRequest<'c> {
    #[inline]
    pub fn builder() -> RestRequestBuilder<'c> {
        RestRequestBuilder::default()
    }
    /// GET request with given method, url, headers and body.
    ///
    /// Jsut a shortcut for `execute` a GET request
    #[inline]
    pub async fn get(self) -> Result<RawResponseExt> {
        self.execute(HttpMethod::GET).await
    }

    /// POST request with given method, url, headers and body.
    ///
    /// Jsut a shortcut for `execute` a POST request
    #[inline]
    pub async fn post(self) -> Result<RawResponseExt> {
        self.execute(HttpMethod::POST).await
    }

    /// Execute request with given method, url, headers and body.
    #[tracing::instrument]
    pub async fn execute(self, method: HttpMethod) -> Result<RawResponseExt> {
        let client = get_client(self.proxy)?;

        let request = {
            let mut r = reqwest::Request::new(method, self.url);
            *r.headers_mut() = self
                .headers
                .unwrap_or_else(|| ManagedHeaderMap::new(false, false).take_inner());
            *r.body_mut() = self.body;
            r
        };

        // SAFE: body is not Stream
        let response = client.execute(request.try_clone().unwrap()).await?;

        Ok(ResponseExt::new(request, self.proxy, (), response))
    }
}

#[derive(Default)]
pub struct RestRequestBuilder<'r> {
    proxy: Option<&'r str>,
    url: Option<&'r str>,
    headers: Option<HttpHeaderMap>,
    body: Option<reqwest::Body>,
}

impl<'c> RestRequestBuilder<'c> {
    /// Configure proxy for the request
    #[inline]
    pub fn proxy(mut self, proxy: Option<&'c str>) -> Self {
        self.proxy = proxy;
        self
    }

    /// Configure url for the request
    #[inline]
    pub fn url(mut self, url: &'c str) -> Self {
        self.url = Some(url);
        self
    }

    /// Configure headers for the request
    ///
    /// DO NOT pass expr `None` directly here, or you have to specify the type of Option<T>
    #[inline]
    pub fn headers(mut self, headers: Option<impl Into<HttpHeaderMap>>) -> Self {
        self.headers = headers.map(|h| h.into());
        self
    }

    /// Configure body for the request
    ///
    /// DO NOT pass expr `None` directly here, or you have to specify the type of Option<T>
    #[inline]
    pub fn body(mut self, body: Option<impl Into<reqwest::Body>>) -> Self {
        self.body = body.map(|b| b.into());
        self
    }

    /// Build RestRequest
    #[inline]
    pub fn build(self) -> Result<RestRequest<'c>> {
        let url = self.url.expect("url is required for RestRequest");
        let url = url.parse().map_err(|e| CrateError::UrlParse {
            url: url.to_owned(),
            source: e,
        })?;

        Ok(RestRequest {
            proxy: self.proxy,
            url,
            headers: self.headers,
            body: self.body,
        })
    }

    /// Build RestRequest with given url
    #[inline]
    pub fn build_with(self, url: Url) -> RestRequest<'c> {
        RestRequest {
            proxy: self.proxy,
            url,
            headers: self.headers,
            body: self.body,
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use lib_utils::headers::ManagedHeaderMap;
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

    #[tokio::test]
    async fn test_keep_alive() {
        tracing_subscriber::registry().with(fmt::layer()).init();

        let url = "https://api.bilibili.com/x/frontend/finger/spi";
        let proxy = "socks5://127.0.0.1:20023";

        let a: Option<ManagedHeaderMap> = None;

        let r = super::RestRequest::builder()
            .proxy(Some(proxy))
            .url(url)
            .headers(a)
            .build()
            .unwrap()
            .get()
            .await
            .unwrap()
            .bili_json()
            .await
            .unwrap();

        tracing::debug!("1 => {:?}", r.data());

        tokio::time::sleep(Duration::from_secs(90)).await;

        let handler = tokio::spawn(async {
            let r = super::RestRequest::builder()
                .proxy(Some(proxy))
                .url(url)
                .build()
                .unwrap()
                .get()
                .await
                .unwrap()
                .bili_json()
                .await
                .unwrap();

            tracing::debug!("SP => {:?}", r.data());
        });

        let r = super::RestRequest::builder()
            .proxy(Some(proxy))
            .url(url)
            .build()
            .unwrap()
            .get()
            .await
            .unwrap()
            .bili_json()
            .await
            .unwrap();

        tracing::debug!("2 => {:?}", r.data());

        let _ = handler.await;
    }
}
