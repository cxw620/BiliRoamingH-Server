use anyhow::{anyhow, Result};
use dashmap::DashMap;
use http_02::{Request as HttpRequest, Response as HttpResponse};

use std::{future::Future, pin::Pin, sync::OnceLock, task::Poll, time::Duration};

use super::{connect_http02::Connector, proxy::Proxy};
use crate::{utils::ManagedHeaderMap, CrateError};

type GrpcClient = hyper_014::Client<Connector, tonic::body::BoxBody>;

static CLIENTS: OnceLock<DashMap<&'static str, GrpcClient>> = OnceLock::new();

/// Init Clients with given proxies url.
///
/// Return error if CLIENTS is already inited.
#[tracing::instrument]
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

#[tracing::instrument]
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
#[tracing::instrument]
pub fn get_client(proxy: Option<&str>) -> Result<GrpcClient> {
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

/// A ClientExt for outgoing gRPC requests.
///
/// Should not reuse this since headers will be taken and cleared after each request.
pub struct GrpcClientExt<'c> {
    proxy: Option<&'c str>,
    headers: ManagedHeaderMap,
    used: bool,
}

impl<'c> GrpcClientExt<'c> {
    #[inline]
    pub fn new(proxy: Option<&'c str>, headers: ManagedHeaderMap) -> Self {
        Self {
            proxy,
            headers,
            used: false,
        }
    }

    #[inline]
    fn headers_mut(&mut self) -> &mut ManagedHeaderMap {
        &mut self.headers
    }
}

type GrpcRequest = HttpRequest<tonic::body::BoxBody>;

impl tower::Service<GrpcRequest> for GrpcClientExt<'_> {
    type Response = HttpResponse<hyper_014::Body>;
    type Error = CrateError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[tracing::instrument(skip(self), level = "debug", name = "GrpcClientExt")]
    fn call(&mut self, mut req: GrpcRequest) -> Self::Future {
        // Deal with original HeaderMap
        let header_map = {
            let header_map_original = req.headers();
            let mut header_map = self.headers_mut().take_inner();

            header_map_original.get("grpc-encoding").map(|encoding| {
                header_map.insert("grpc-encoding", encoding.clone());
            });
            header_map_original
                .get("grpc-accept-encoding")
                .map(|accept_encoding| {
                    header_map.insert("grpc-accept-encoding", accept_encoding.clone());
                });

            header_map
        };
        *req.headers_mut() = header_map;

        let client = get_client(self.proxy);

        if self.used {
            tracing::error!("GrpcClientExt should not be reused");
            return Box::pin(async move { Err(CrateError::Unknown) });
        }

        Box::pin(async move {
            let client = client?;

            let mut response = client.request(req).await.map_err(|e| {
                tracing::error!("GrpcClient met with error: {:?}", e);
                CrateError::from(e)
            })?;

            let headers = response.headers_mut();

            let status = tonic::Status::from_header_map(headers).ok_or_else(|| {
                tracing::error!("GrpcClient met with error: not Grpc Response");
                CrateError::NotGrpcResponse
            })?;

            // Remove gRPC status headers, or tonic will complain about "protocol error:
            // received message with compressed-flag but no grpc-encoding was specified"
            headers.remove("grpc-status");
            headers.remove("grpc-message");
            headers.remove("grpc-status-details-bin");

            if status.code() != tonic::Code::Ok {
                tracing::error!("GrpcClient met with error: {:?}", status);
                return Err(CrateError::from(status));
            }

            Ok(response)
        })
    }
}

#[cfg(test)]
mod test {
    use tonic::IntoRequest;

    use std::time::Duration;

    use super::GrpcClientExt;
    use lib_bilibili::bapis::{
        app::playerunite::v1::{player_client::PlayerClient, PlayViewUniteReq},
        metadata::device::Device,
        playershared::VideoVod,
    };
    use lib_utils::{
        av2bv,
        headers::{BiliHeaderT, ManagedHeaderMap},
        now,
    };

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

    #[tracing::instrument]
    async fn test_custom_client(proxy: Option<&str>) {
        let uri = http_02::Uri::from_static("https://app.bilibili.com");
        let headers = headers();
        let mut client = PlayerClient::with_origin(GrpcClientExt::new(proxy, headers), uri)
            .accept_compressed(tonic::codec::CompressionEncoding::Gzip)
            .send_compressed(tonic::codec::CompressionEncoding::Gzip);

        let now = now!().as_millis();

        let request = request_for_test();
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
        use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
        tracing_subscriber::registry().with(fmt::layer()).init();

        println!("============ Custom Client ============");
        test_custom_client(Some("http://127.0.0.1:20033")).await;
        tokio::time::sleep(Duration::from_secs(30)).await;

        let h1 = tokio::spawn(async {
            test_custom_client(Some("http://127.0.0.1:20033")).await;
        });

        let h2 = tokio::spawn(async {
            test_custom_client(Some("http://127.0.0.1:20033")).await;
        });

        let h3 = tokio::spawn(async {
            test_custom_client(Some("http://127.0.0.1:20033")).await;
        });

        let h4 = tokio::spawn(async {
            test_custom_client(Some("http://127.0.0.1:20033")).await;
        });

        let h5 = tokio::spawn(async {
            test_custom_client(Some("http://127.0.0.1:20033")).await;
        });

        let h6 = tokio::spawn(async {
            test_custom_client(Some("http://127.0.0.1:20033")).await;
        });

        // test_custom_client(Some("http://127.0.0.1:20033")).await;
        // test_custom_client(Some("http://127.0.0.1:20033")).await;
        // test_custom_client(Some("http://127.0.0.1:20033")).await;
        // test_custom_client(Some("http://127.0.0.1:20033")).await;
        // test_custom_client(Some("http://127.0.0.1:20033")).await;
        // test_custom_client(Some("http://127.0.0.1:20033")).await;
        // test_custom_client(Some("http://127.0.0.1:20033")).await;
        // test_custom_client(Some("http://127.0.0.1:20033")).await;
        // test_custom_client(Some("http://127.0.0.1:20033")).await;

        let _ = tokio::join!(h1, h2, h3, h4, h5, h6);
    }
}
