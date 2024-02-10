use anyhow::{anyhow, Result};
use dashmap::DashMap;
use reqwest::{Client, Proxy};
use std::{sync::OnceLock, time::Duration};

use crate::{CrateError, utils::RawResponseExt};

/// Clients with or without proxy
static CLIENTS: OnceLock<DashMap<&'static str, reqwest::Client>> = OnceLock::new();

/// Init Clients with given proxies url.
///
/// Return error if CLIENTS is already inited.
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
async fn get_client(proxy: Option<&str>) -> Result<reqwest::Client> {
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

pub async fn get<T, H>(url: T, proxy: Option<&str>, headers: Option<H>) -> Result<RawResponseExt>
where
    T: TryInto<url::Url>,
    T::Error: Into<url::ParseError>,
    H: Into<reqwest::header::HeaderMap>,
{
    todo!()
}

pub async fn post<T, H, B>(
    url: T,
    proxy: Option<&str>,
    headers: Option<H>,
    body: Option<B>,
) -> Result<RawResponseExt>
where
    T: TryInto<url::Url>,
    T::Error: Into<url::ParseError>,
    H: Into<reqwest::header::HeaderMap>,
    B: Into<reqwest::Body>,
{
    todo!()
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

    use crate::utils::ResponseExt;

    #[tokio::test]
    async fn test() {
        tracing_subscriber::registry().with(fmt::layer()).init();

        let url = url::Url::try_from("https://api.bilibili.com/x/frontend/finger/spi").unwrap();
        // let req = client.get(
        //     url
        // ).header("user-agent", "Dalvik/2.1.0 (Linux; U; Android 13; NOH-AN02 Build/HUAWEINOH-AN02) 7.9.0 os/android model/NOH-AN02 mobi_app/android_i build/7090300 channel/master innerVer/7090300 osVer/13 network/1").build().unwrap();
        // let resp = client.execute(req.try_clone().unwrap()).await.unwrap();
        // let resp_ext = ResponseExt::new(req, None, (), resp)
        //     .into_bili_result()
        //     .await
        //     .unwrap();
        // println!("{:?}", resp_ext.o_req());
        // println!("{:?}", resp_ext.o_req().version());
        // println!("{:?}", resp_ext.o_proxy());
        // println!("{:?}", resp_ext.headers());
        // println!("{:?}", resp_ext.data());

        let url_clone = url.clone();
        let _ = tokio::spawn(async move {
            let client = super::get_client(Some("socks5://127.0.0.1:20023"))
                .await
                .unwrap();
            let resp = client.clone().get(url_clone).header("user-agent", "Dalvik/2.1.0 (Linux; U; Android 13; NOH-AN02 Build/HUAWEINOH-AN02) 7.9.0 os/android model/NOH-AN02 mobi_app/android_i build/7090300 channel/master innerVer/7090300 osVer/13 network/1").send().await.unwrap();
            tracing::warn!("1 => {:?}", resp.text().await);
        }).await;

        tokio::time::sleep(Duration::from_secs(300)).await;

        let url_clone = url.clone();
        let _ = tokio::spawn(async move {
            let client = super::get_client(Some("socks5://127.0.0.1:20023"))
                .await
                .unwrap();
            let resp = client.clone().get(url_clone).header("user-agent", "Dalvik/2.1.0 (Linux; U; Android 13; NOH-AN02 Build/HUAWEINOH-AN02) 7.9.0 os/android model/NOH-AN02 mobi_app/android_i build/7090300 channel/master innerVer/7090300 osVer/13 network/1").send().await.unwrap();
            tracing::warn!("2 => {:?}", resp.text().await);
        }).await;

        // tokio::
    }
}
