use anyhow::Result;
use http_02::HeaderMap as HttpHeaderMap;
use lib_utils::error::ServerErrorExt;

use super::{
    bapis::app::playerunite::v1::{
        player_client::PlayerClient, PlayViewUniteReply, PlayViewUniteReq,
    },
    client::grpc::{client_http02::GrpcClientExt, CompressionEncoding},
    interface::RpcBuilderT,
};
use crate::{
    model::{playurl::PlayurlReq, response::ResponseWrapper},
    utils::{ManagedHeaderMap, Upstream},
};

#[derive(Debug)]
/// RPC builder for Playurl
pub struct PlayurlRpc<'r> {
    upstream: Upstream<'r>,
    proxy: Option<&'r str>,
    headers: ManagedHeaderMap,
    request: PlayurlReq<'r>,
}

impl<'r> RpcBuilderT<'r> for PlayurlRpc<'r> {
    const DEFAULT_UPSTREAM: Upstream<'r> = Upstream::APP_DEFAULT;

    type Request = PlayurlReq<'r>;
    type Response = ResponseWrapper<PlayViewUniteReply>;

    #[inline]
    fn new(request: Self::Request, upstream: impl Into<Upstream<'r>>) -> Self {
        Self {
            upstream: upstream.into(),
            proxy: None,
            headers: ManagedHeaderMap::new(true, true),
            request,
        }
    }

    #[inline]
    fn with_upstream(mut self, upstream: impl Into<Upstream<'r>>) -> Self {
        self.upstream = upstream.into();
        self
    }

    #[inline]
    fn with_proxy(mut self, proxy: Option<&'r str>) -> Self {
        self.proxy = proxy;
        self
    }

    #[inline]
    fn with_headers(mut self, headers: Option<impl Into<HttpHeaderMap>>) -> Self {
        if let Some(headers) = headers {
            self.headers = ManagedHeaderMap::new_from_existing(headers.into(), true, true);
        }
        self
    }

    #[inline]
    fn with_headers_managed(mut self, headers: Option<impl Into<ManagedHeaderMap>>) -> Self {
        if let Some(headers) = headers {
            self.headers = headers.into();
        }
        self
    }

    #[tracing::instrument(level = "debug", name = "PlayurlRpc.execute", err)]
    async fn execute(self) -> Result<ResponseWrapper<PlayViewUniteReply>> {
        let request: PlayViewUniteReq = self.request.try_into()?;

        let grpc_client = GrpcClientExt::new(self.proxy, self.headers);

        let mut client = PlayerClient::with_origin(grpc_client, self.upstream.uri()?)
            .accept_compressed(CompressionEncoding::Gzip);
            // .send_compressed(CompressionEncoding::Gzip);

        client
            .play_view_unite(request)
            .await
            .map(|r| {
                let (headers, inner, _) = r.into_parts();
                ResponseWrapper {
                    inner,
                    headers: headers.into_headers(),
                }
            })
            .map_err(|e| ServerErrorExt::from(e).into())
    }
}
