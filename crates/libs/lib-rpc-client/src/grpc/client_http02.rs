use anyhow::{anyhow, Result};
use dashmap::DashMap;

use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use super::{connect_http02::Connector, proxy::Proxy};

type GrpcClient = hyper_014::Client<Connector, tonic::body::BoxBody>;

static CLIENTS: OnceLock<DashMap<&'static str, GrpcClient>> = OnceLock::new();

/// Init Clients with given proxies url.
///
/// Return error if CLIENTS is already inited.
pub fn init_grpc_client(proxies: Vec<&'static str>) -> Result<()> {
    let map = DashMap::with_capacity(16);

    // Default client without proxy
    map.insert("default", gen_client(None)?);

    for p in proxies {
        let rp = Proxy::new(p)?;
        map.insert(p, gen_client(Some(rp))?);
    }

    CLIENTS.set(map).map_err(|_| {
        tracing::error!("CLIENTS should be initialized only once");
        anyhow!("CLIENTS should be initialized only once")
    })?;

    Ok(())
}

fn gen_client(proxies: Option<Proxy>) -> Result<GrpcClient> {
    let proxies = proxies.map_or(vec![], |p| vec![p]);

    let connector = Connector::new(proxies, None);

    let client = hyper_014::Client::builder()
        .pool_idle_timeout(Duration::from_secs(3600))
        .http2_keep_alive_interval(Some(Duration::from_secs(18)))
        .http2_keep_alive_while_idle(true)
        .http2_keep_alive_timeout(Duration::from_secs(16))
        .http2_initial_connection_window_size(Some((1 << 28) - 1))
        .http2_initial_stream_window_size(Some((1 << 28) - 1))
        .build(connector);

    Ok(client)
}

/// Get GrpcClient from CLIENTS cache or new one with given proxy
pub async fn get_client(proxy: Option<&str>) -> Result<GrpcClient> {
    let clients = CLIENTS.get_or_init(|| {
        tracing::warn!("CLIENTS should be initialized before get_client!!!");
        let map = DashMap::with_capacity(16);
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
        tracing::trace!("Got GrpcClient from cache");
        Ok(client)
    } else {
        tracing::warn!("Unknown given proxy, Box::leak may cause memory leak");

        let proxy_str = proxy.unwrap();

        let rp = Proxy::new(proxy_str)?;

        let client = gen_client(Some(rp))?;
        clients.insert(Box::leak(Box::new(proxy_str.to_string())), client.clone());

        tracing::trace!("Got new GrpcClient from given proxy [{:?}]", proxy);
        Ok(client)
    }
}

#[cfg(test)]
mod test {
    use lib_bilibili::bapis::metadata::device::Device;
    use lib_bilibili::bapis::{
        app::playerunite::v1::{player_client::PlayerClient, PlayViewUniteReq},
        playershared::VideoVod,
    };
    use lib_utils::{
        av2bv,
        headers::{BiliHeaderT, ManagedHeaderMap},
        now,
    };

    use crate::grpc::connect_http02::Connector;
    use crate::grpc::proxy::Proxy;
    use std::time::Duration;
    use tonic::transport::Channel;
    use tonic::IntoRequest;

    fn request_for_test() -> tonic::Request<PlayViewUniteReq> {
        let mut request = PlayViewUniteReq {
            vod: Some(VideoVod {
                aid: 80433022,
                cid: 137649199,
                qn: 126,
                fnval: 16 ^ 64 ^ 128 ^ 256 ^ 512 ^ 1024 ^ 2048,
                force_host: 2,
                fourk: true,
                prefer_codec_type: 2,
                download: 2,
                ..Default::default()
            }),
            bvid: av2bv!(80433022),
            ..Default::default()
        }
        .into_request();

        let _ = std::mem::replace(request.metadata_mut(), headers().into());

        request
    }

    fn connector(proxy_str: Option<&str>) -> Connector {
        let mut proxies = vec![];
        if let Some(proxy_str) = proxy_str {
            let proxy = Proxy::new(proxy_str).unwrap();
            proxies.push(proxy);
        }

        Connector::new(proxies, None)
    }

    fn headers() -> ManagedHeaderMap {
        let mut headers = ManagedHeaderMap::new(true, true);

        headers
            .set_user_agent(None)
            .set_access_key("da656a29342088bbfd134af49d28ef21")
            .set_appkey_name("android64")
            // .set_buvid("buvid")
            .set_device_bin(Device::default());

        // tracing::debug!("{:?}", &headers);

        headers
    }

    async fn channel_with_proxy(proxy_str: Option<&str>) -> Channel {
        Channel::builder("https://app.bilibili.com".parse().unwrap())
            .tcp_keepalive(Some(Duration::from_secs(3600)))
            .http2_keep_alive_interval(Duration::from_secs(18))
            .keep_alive_while_idle(true)
            .keep_alive_timeout(Duration::from_secs(16))
            .initial_connection_window_size(Some((1 << 28) - 1))
            .initial_stream_window_size(Some((1 << 28) - 1))
            .connect_with_connector(connector(proxy_str))
            .await
            .unwrap()
    }

    async fn hyper_client(
        connector: Connector,
    ) -> hyper_014::Client<Connector, tonic::body::BoxBody> {
        hyper_014::Client::builder()
            .pool_idle_timeout(Duration::from_secs(3600))
            // ! Should set UA separately
            // .user_agent(user_agent)
            .http2_keep_alive_interval(Some(Duration::from_secs(18)))
            .http2_keep_alive_while_idle(true)
            .http2_keep_alive_timeout(Duration::from_secs(16))
            .http2_initial_connection_window_size(Some((1 << 28) - 1))
            .http2_initial_stream_window_size(Some((1 << 28) - 1))
            .build(connector)
    }

    #[tokio::test]
    async fn test_grpc() {
        let request = request_for_test();

        let tls = crate::grpc::tls::rustls_config(true);

        let mut http = hyper_014::client::connect::HttpConnector::new();
        http.enforce_http(false);

        // We have to do some wrapping here to map the request type from
        // `https://example.com` -> `https://[::1]:50051` because `rustls`
        // doesn't accept ip's as `ServerName`.
        let connector = tower::ServiceBuilder::new()
            .layer_fn(move |s| {
                let tls = tls.clone();

                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_tls_config(tls)
                    .https_or_http()
                    .enable_http2()
                    .wrap_connector(s)
            })
            // Since our cert is signed with `example.com` but we actually want to connect
            // to a local server we will override the Uri passed from the `HttpsConnector`
            // and map it to the correct `Uri` that will connect us directly to the local server.
            .map_request(|_| http_02::Uri::from_static("https://app.bilibili.com"))
            .service(http);

        let client = hyper_014::Client::builder().build(connector);

        // Using `with_origin` will let the codegenerated client set the `scheme` and
        // `authority` from the porvided `Uri`.
        let uri = http_02::Uri::from_static("https://app.bilibili.com");

        let mut client = PlayerClient::with_origin(client, uri)
            .accept_compressed(tonic::codec::CompressionEncoding::Gzip)
            .send_compressed(tonic::codec::CompressionEncoding::Gzip);

        let now = now!().as_millis();

        let resp = client.play_view_unite(request).await.unwrap();

        println!("time: {}", now!().as_millis() - now);
        println!("{:?}", resp);
    }

    async fn test_grpc_with_channel(channel: Channel) {
        let request = request_for_test();

        let mut client = PlayerClient::new(channel)
            .accept_compressed(tonic::codec::CompressionEncoding::Gzip)
            .send_compressed(tonic::codec::CompressionEncoding::Gzip);

        let now = now!().as_millis();

        let _resp = client
            .play_view_unite(request)
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e
            })
            .unwrap();

        println!("time: {}", now!().as_millis() - now);
        // println!("{:?}", resp);
    }

    async fn test_grpc_with_interceptor(
        client: hyper_014::Client<Connector, tonic::body::BoxBody>,
    ) {
        let request = request_for_test();
        let headers = headers();

        let mut client = PlayerClient::with_interceptor(
            tower::service_fn(move |mut req: hyper_014::Request<tonic::body::BoxBody>| {
                let mut parts = std::mem::take(req.uri_mut()).into_parts();
                parts.scheme = Some(http_02::uri::Scheme::HTTPS);
                parts.authority = Some(http_02::uri::Authority::from_static("app.bilibili.com"));
                let uri = http_02::Uri::from_parts(parts).unwrap();
                let _ = std::mem::replace(req.uri_mut(), uri);
                client.request(req)
            }),
            headers,
        )
        .accept_compressed(tonic::codec::CompressionEncoding::Gzip)
        .send_compressed(tonic::codec::CompressionEncoding::Gzip);

        let now = now!().as_millis();

        let _resp = client
            .play_view_unite(request)
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e
            })
            .unwrap();

        println!("time: {}", now!().as_millis() - now);
        // println!("{:?}", resp);
    }

    #[tokio::test]
    async fn test() {
        // use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
        // tracing_subscriber::registry().with(fmt::layer()).init();
        println!("============= Channel =============");

        let channel = channel_with_proxy(Some("http://127.0.0.1:20033")).await;

        test_grpc_with_channel(channel.clone()).await;
        tokio::time::sleep(Duration::from_secs(30)).await;
        test_grpc_with_channel(channel.clone()).await;
        test_grpc_with_channel(channel.clone()).await;
        test_grpc_with_channel(channel.clone()).await;
        test_grpc_with_channel(channel.clone()).await;
        test_grpc_with_channel(channel.clone()).await;
        test_grpc_with_channel(channel.clone()).await;
        test_grpc_with_channel(channel.clone()).await;
        test_grpc_with_channel(channel.clone()).await;
        test_grpc_with_channel(channel.clone()).await;
        test_grpc_with_channel(channel.clone()).await;

        println!("============ Interceptor ============");

        let connector = connector(Some("http://127.0.0.1:20033"));
        let client = hyper_client(connector).await;
        test_grpc_with_interceptor(client.clone()).await;
        tokio::time::sleep(Duration::from_secs(30)).await;
        test_grpc_with_interceptor(client.clone()).await;
        test_grpc_with_interceptor(client.clone()).await;
        test_grpc_with_interceptor(client.clone()).await;
        test_grpc_with_interceptor(client.clone()).await;
        test_grpc_with_interceptor(client.clone()).await;
        test_grpc_with_interceptor(client.clone()).await;
        test_grpc_with_interceptor(client.clone()).await;
        test_grpc_with_interceptor(client.clone()).await;
        test_grpc_with_interceptor(client.clone()).await;
        test_grpc_with_interceptor(client.clone()).await;
    }
}
