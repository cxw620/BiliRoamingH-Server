// use anyhow::anyhow;
// use std::borrow::Cow;

// use crate::parse_field;

// /// Playurl Related Request info
// pub struct PlayurlReq<'q> {
//     pub vod: VideoVod,
//     pub vod_ext: VideoVodExt<'q>,
// }

// /// Video VOD Info
// pub struct VideoVod {
//     /// 视频aid
//     pub aid: i32,
//     /// 视频cid
//     pub cid: i32,
//     /// 清晰度
//     pub qn: u64,
//     /// 视频流版本
//     pub fnver: i32,
//     /// 视频流格式
//     pub fnval: i32,
//     /// 下载模式
//     /// 0:播放 1:flv下载 2:dash下载
//     pub download: u32,
//     /// 流url强制是用域名
//     /// 0:允许使用ip 1:使用http 2:使用https
//     pub force_host: i32,
//     /// 是否4K
//     pub fourk: bool,
//     /// 视频编码
//     pub prefer_codec_type: CodeType,
//     /// 响度均衡
//     pub voice_balance: u64,
// }

// impl Default for VideoVod {
//     fn default() -> Self {
//         Self {
//             aid: -1,
//             cid: -1,
//             qn: 127,
//             fnver: 0,
//             fnval: 4048,
//             download: 0,
//             force_host: 2,
//             fourk: true,
//             prefer_codec_type: CodeType::Code265,
//             voice_balance: 0,
//         }
//     }
// }

// /// 视频编码
// pub enum CodeType {
//     /// 不指定
//     Nocode = 0,
//     /// H264
//     Code264 = 1,
//     /// H265
//     Code265 = 2,
//     /// AV1
//     Codeav1 = 3,
// }

// pub struct VideoVodExt<'v> {
//     pub bvid: Option<Cow<'v, str>>,
//     pub ep_id: Option<i64>,
//     pub season_id: Option<i64>,
//     pub web_location: Option<Cow<'v, str>>,
// }

// use lib_utils::url::QueryMap;



// impl<'m, 'q> TryFrom<QueryMap<'m>> for PlayurlReq<'q> {
//     type Error = anyhow::Error;
//     fn try_from(m: QueryMap) -> Result<Self, Self::Error> {
//         // let aid = m.get("aid").unwrap_or_else(|| m.get("bvid"));
//         let vod = VideoVod {
//             aid: parse_field!(m, "aid"),
//             cid: parse_field!(m, "cid"),
//             qn: parse_field!(m, "qn", u64, 127),
//             fnver: parse_field!(m, "fnver", i32, 0),
//             fnval: parse_field!(m, "fnval", i32, 4048),
//             download: parse_field!(m, "download", u32, 0),
//             force_host: parse_field!(m, "force_host", i32, 0),
//             fourk: {
//                 let fourk = m.get("fourk").unwrap_or("1");
//                 fourk == "1" || fourk == "true"
//             },
//             voice_balance: parse_field!(m, "voice_balance", u64, 0),
//             ..Default::default()
//         };
//         let vod_ext = todo!();
//         Ok(Self { vod, vod_ext })
//     }
// }
