use anyhow::{anyhow, Result};

use std::{borrow::Cow, collections::HashMap};

use crate::request::bapis::app::playerunite::v1::PlayViewUniteReq;

// Re-export VideoVod
pub use crate::request::bapis::playershared::VideoVod;

/// Playurl Related Request info
#[derive(Debug)]
pub struct PlayurlReq<'q> {
    pub vod: VideoVod,
    pub vod_ext: VideoVodExt<'q>,
}

#[derive(Debug, Default)]
pub struct VideoVodExt<'q> {
    /// Video BVID, leave empty when generated with aid
    pub bvid: Option<Cow<'q, str>>,
    /// PGC / PUGC Content: ep_id
    pub ep_id: Option<Cow<'q, str>>,
    /// PGC / PUGC Content: season_id
    pub season_id: Option<Cow<'q, str>>,
    /// PGC / PUGC Content: media_id
    pub media_id: Option<Cow<'q, str>>,
}

use lib_utils::{av2bv, bv2av, error::ServerError, parse_field, url::QueryMap};

impl<'q, 'm: 'q> TryFrom<&'m QueryMap<'m>> for PlayurlReq<'q> {
    type Error = anyhow::Error;

    #[tracing::instrument(
        level = "debug",
        name = "model.playurl.PlayurlReq.try_from QueryMap",
        err
    )]
    fn try_from(m: &'m QueryMap<'m>) -> Result<Self> {
        let (aid, bvid) = if m.get("ep_id").is_some() {
            // PGC content with ep_id, aid or bvid is not a must
            (parse_field!(m, "aid", 0), m.get("bvid").map(Cow::Borrowed))
        } else {
            m.get("aid")
                .map(|aid| {
                    aid.parse::<i32>()
                        .map(|aid| (aid, m.get("bvid").map(|bvid| Cow::Borrowed(bvid))))
                        .map_err(|e| anyhow!(e))
                })
                .unwrap_or_else(|| {
                    let bvid = m.get("bvid").ok_or(ServerError::FatalReqParamMissing)?;
                    Ok((bv2av!(bvid) as i32, Some(bvid.into())))
                })?
        };

        let vod = VideoVod {
            aid,
            cid: parse_field!(m, "cid"),
            qn: parse_field!(m, "qn", 127), // 8K
            fnver: parse_field!(m, "fnver", 0),
            fnval: parse_field!(m, "fnval", 4048), // 16 ^ 64 ^ 128 ^ 256 ^ 512 ^ 1024 ^ 2048
            download: parse_field!(m, "download", 0),
            force_host: parse_field!(m, "force_host", 0),
            fourk: parse_field!(m, "fourk", 1) == 1,
            prefer_codec_type: parse_field!(m, "prefer_codec_type", 0),
            voice_balance: parse_field!(m, "voice_balance", u64, 0),
        };
        let vod_ext = VideoVodExt {
            bvid,
            ep_id: m.get("ep_id").map(Cow::Borrowed),
            season_id: m.get("season_id").map(Cow::Borrowed),
            media_id: m.get("media_id").map(Cow::Borrowed),
        };
        Ok(Self { vod, vod_ext })
    }
}

impl<'r> TryFrom<PlayViewUniteReq> for PlayurlReq<'r> {
    type Error = anyhow::Error;

    fn try_from(mut req: PlayViewUniteReq) -> Result<Self> {
        let vod = req.vod.ok_or(ServerError::GeneralRpc)?;

        let vod_ext = VideoVodExt {
            bvid: Some(Cow::Owned(req.bvid)),
            ep_id: req.extra_content.remove("ep_id").map(Cow::Owned),
            season_id: req.extra_content.remove("season_id").map(Cow::Owned),
            media_id: req.extra_content.remove("media_id").map(Cow::Owned),
        };
        Ok(Self { vod, vod_ext })
    }
}

impl<'r> TryFrom<PlayurlReq<'r>> for PlayViewUniteReq {
    type Error = anyhow::Error;

    fn try_from(req: PlayurlReq<'r>) -> Result<Self> {
        let bvid = req
            .vod_ext
            .bvid
            .map(|v| v.into_owned())
            .unwrap_or_else(|| av2bv!(req.vod.aid as u64));

        let mut extra_content = HashMap::with_capacity(3);
        if let Some(ep_id) = req.vod_ext.ep_id {
            extra_content.insert("ep_id".to_string(), ep_id.into_owned());
        }
        if let Some(season_id) = req.vod_ext.season_id {
            extra_content.insert("season_id".to_string(), season_id.into_owned());
        }
        if let Some(media_id) = req.vod_ext.media_id {
            extra_content.insert("media_id".to_string(), media_id.into_owned());
        }

        let req = Self {
            vod: Some(req.vod),
            bvid,
            extra_content,
            ..Default::default()
        };
        Ok(req)
    }
}
