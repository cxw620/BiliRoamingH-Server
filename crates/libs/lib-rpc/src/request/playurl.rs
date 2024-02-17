// use anyhow::Result;
// use lib_utils::headers::{self, ManagedHeaderMap};

// use crate::{
//     request::{
//         bapis::app::playerunite::v1::{
//             player_client::PlayerClient, PlayViewUniteReply, PlayViewUniteReq,
//         },
//         client::grpc::client_http02::GrpcClientExt,
//     },
//     utils::{Upstream, UpstreamType},
// };

// use crate::utils::BiliArea;



// pub struct PlayurlV1<'p> {
//     access_key: Option<&'p str>
    
// }



// #[tracing::instrument]
// /// bilibili.app.playerunite.playview
// pub async fn bilibili_app_playerunite_playviewunite(
//     area: impl Into<BiliArea> + std::fmt::Debug,
// ) -> Result<PlayViewUniteReply> {
//     let upstream = UpstreamType::AppBilibiliCom
//         .upstream()
//         .with_custom("https://app.bilibili.com")
//         .uri()?;

//     let headers = ManagedHeaderMap::new(true, true);

//     let proxy = Some("socks5://127.0.0.1:20023");

//     let client = GrpcClientExt::new(proxy, headers);

//     let client = PlayerClient::with_origin(client, upstream);

//     todo!()
// }
