use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::error;

use lib_utils::error::ServerError;
use lib_utils::parse_grpc_any;

/// Pgc Playurl Reply for compatibility
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PgcPlayurlReply {
    /// 状态码 (不重要)
    pub code: i64,
    /// 留空 (不重要)
    pub message: String,
    /// ? (不重要)
    pub status: i32,
    /// 默认 `suee` (不重要)
    pub result: String,
    /// 接受的视频格式(不重要)
    pub accept_format: String,
    /// 暂恒定为 `start` (不重要)
    pub seek_param: String,
    /// 是否为预览
    ///
    /// PlayViewBusinessInfo.is_preview
    pub is_preview: i32,
    /// VideoVod.fnval
    pub fnval: i32,
    /// 视频是否可投影?, 暂恒定为 true
    ///
    /// VodInfo.support_project
    pub video_project: bool,
    /// VideoVod.fnver 暂恒定为 0
    pub fnver: i32,
    /// 暂恒定为 `DASH`, `FLV` or `MP4` is not supported.
    pub r#type: String,
    /// 用户是否承包
    ///
    /// PlayViewBusinessInfo.bp
    pub bp: i32,
    /// 默认 `offset` (不重要)
    pub seek_type: String,
    /// 大会员类型 (不重要)
    pub vip_type: i32,
    /// 默认 `local` (不重要)
    pub from: String,
    /// 视频编码id
    ///
    /// VodInfo.video_codecid
    pub video_codecid: u32,
    /// 备案登记信息
    ///
    /// PlayViewBusinessInfo.record_info
    pub record_info: RecordInfo,
    /// 是否 DRM 限制, 默认 false
    ///
    /// PlayViewBusinessInfo.is_drm
    pub is_drm: bool,
    /// 是否非全二压, 默认 0 (不重要)
    ///
    /// PlayViewBusinessInfo.no_rexcode
    pub no_rexcode: i32,
    /// 视频格式
    ///
    /// VodInfo.format
    pub format: String,
    /// 视频流支持的格式
    ///
    /// Generated with StreamInfo
    pub support_formats: Vec<SupportFormat>,
    /// 视频流存在的视频清晰度
    pub accept_quality: Vec<u32>,
    /// 默认视频清晰度
    pub quality: u32,
    /// 视频流长度 (sec)
    pub timelength: u64,
    /// 视频流 (SegmentVideo)
    ///
    /// NOT SUPPORTED
    pub durls: Vec<Value>,
    /// 已付费
    pub has_paid: bool,
    /// 大会员状态
    ///
    /// PlayViewBusinessInfo.vip_status
    pub vip_status: i32,
    /// 音视频流 (DASH 类型)
    pub dash: Option<VodDash>,
    /// 跳过片头/片尾配置, 留空?
    pub clip_info_list: Vec<Value>,
    /// 留空 (不重要)
    pub accept_description: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordInfo {
    pub record_icon: String,
    pub record: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportFormat {
    /// StreamInfo.display_desc
    pub display_desc: String,
    /// 留空 (不重要)
    pub sub_description: String,
    /// StreamInfo.superscript
    pub superscript: String,
    /// StreamInfo.need_login
    pub need_login: bool,
    /// 留空 (不重要)
    pub codecs: Vec<String>,
    /// StreamInfo.format
    pub format: String,
    /// StreamInfo.description
    pub description: String,
    /// StreamInfo.need_vip
    pub need_vip: bool,
    /// StreamInfo.quality
    pub quality: u32,
    /// StreamInfo.new_description
    pub new_description: String,
}

use lib_bilibili::bapis::playershared::StreamInfo;
impl From<StreamInfo> for SupportFormat {
    fn from(stream_info: StreamInfo) -> Self {
        Self {
            display_desc: stream_info.display_desc,
            superscript: stream_info.superscript,
            need_login: stream_info.need_login,
            format: stream_info.format,
            description: stream_info.description,
            need_vip: stream_info.need_vip,
            quality: stream_info.quality,
            new_description: stream_info.new_description,
            ..Default::default()
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VodDash {
    pub video: Vec<DashItem>,
    pub audio: Vec<DashItem>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashItem {
    /// 清晰度
    pub id: u32,
    /// 主线流
    pub base_url: String,
    /// 备用流
    pub backup_url: Vec<String>,
    /// 带宽
    pub bandwidth: u32,
    /// 编码id
    pub codecid: u32,
    /// md5
    pub md5: String,
    /// 视频大小
    pub size: u64,
    /// 帧率
    pub frame_rate: String,
}

use lib_bilibili::bapis::playershared::DashItem as PlaysharedDashItem;
impl From<PlaysharedDashItem> for DashItem {
    fn from(item: PlaysharedDashItem) -> Self {
        Self {
            id: item.id,
            base_url: item.base_url,
            backup_url: item.backup_url,
            bandwidth: item.bandwidth,
            codecid: item.codecid,
            md5: item.md5,
            size: item.size,
            frame_rate: item.frame_rate,
        }
    }
}

use lib_bilibili::bapis::app::playerunite::v1::PlayViewUniteReply;
impl TryFrom<PlayViewUniteReply> for PgcPlayurlReply {
    type Error = anyhow::Error;

    #[tracing::instrument(
        level = "debug",
        name = "service.model.playurl_compat.PgcPlayurlReply.try_from PlayViewUniteReply",
        err
    )]
    fn try_from(reply: PlayViewUniteReply) -> Result<Self, Self::Error> {
        let vod_info = reply.vod_info.ok_or_else(|| {
            error!("PlayViewUniteReply.vod_info is None");
            anyhow!(ServerError::General)
        })?;

        let mut no_rexcode = false;
        let mut support_formats = Vec::with_capacity(8);

        // Video DashItem
        // Need sorting by quality?
        let video_dash = vod_info
            .stream_list
            .into_iter()
            .filter_map(|s| {
                use lib_bilibili::bapis::playershared::stream::Content;
                match s.content? {
                    Content::DashVideo(dash_video) => {
                        let stream_info = s.stream_info?;

                        // ECode Should be 0?
                        if stream_info.err_code != 0 {
                            error!(
                                "PlayViewUniteReply.vod_info.stream_list.content.stream_info.err_code is not 0, actually [{}]",
                                stream_info.err_code
                            );
                            return None;
                        }

                        let id = stream_info.quality;
                        no_rexcode = stream_info.no_rexcode;
                        support_formats.push(SupportFormat::from(stream_info));
                        Some(DashItem {
                            id,
                            base_url: dash_video.base_url,
                            backup_url: dash_video.backup_url,
                            bandwidth: dash_video.bandwidth,
                            codecid: dash_video.codecid,
                            md5: dash_video.md5,
                            size: dash_video.size,
                            frame_rate: dash_video.frame_rate,
                        })
                    }
                    _ => {
                        error!("PlayViewUniteReply.vod_info.stream_list.content is not DashVideo");
                        None
                    }
                }
            })
            .collect();

        let accept_quality = support_formats.iter().map(|item| item.quality).collect();

        let audio_dash = vod_info
            .dash_audio
            .into_iter()
            .map(|item| DashItem::from(item))
            .collect();

        let supplement = reply.supplement.ok_or_else(|| {
            error!("PlayViewUniteReply.supplement is None");
            anyhow!(ServerError::General)
        })?;
        let pgc_any_model = if supplement.type_url
            == "type.googleapis.com/bilibili.app.playerunite.pgcanymodel.PGCAnyModel"
        {
            parse_grpc_any!(
                &supplement.value[..],
                lib_bilibili::bapis::app::playerunite::pgcanymodel::PgcAnyModel
            )
        } else {
            error!("PlayViewUniteReply.supplement parse gRPC any error: not [PGCAnyModel], actually [{}]", supplement.type_url);
            bail!(ServerError::General)
        };

        let playview_business_info = pgc_any_model.business.ok_or_else(|| {
            error!("PlayViewUniteReply.supplement.business is None");
            anyhow!(ServerError::General)
        })?;

        let play_arc = reply.play_arc.unwrap_or_default();

        let result = Self {
            code: 0,
            // message: "".to_owned(),
            status: 2,
            result: "suee".to_owned(),
            // accept_format: "".to_owned(),
            seek_param: "start".to_owned(),
            is_preview: play_arc.is_preview as i32, // Not known exactly
            fnval: 4048,                            // Set to 4048?
            video_project: true,
            // fnver: 0,
            r#type: "DASH".to_string(),
            bp: playview_business_info.bp as i32,
            seek_type: "offset".to_string(),
            // vip_type: 0, // TODO Get from cache?
            from: "local".to_string(),
            video_codecid: vod_info.video_codecid,
            // record_info: RecordInfo::default(),
            is_drm: playview_business_info.is_drm, // TODO: DRM need check more!
            no_rexcode: no_rexcode as i32,
            format: vod_info.format,
            support_formats,
            accept_quality,
            quality: vod_info.quality,
            timelength: vod_info.timelength,
            // durls: Vec::new(),
            has_paid: playview_business_info
                .user_status
                .unwrap_or_default()
                .pay_check,
            vip_status: playview_business_info.vip_status,
            dash: Some(VodDash {
                video: video_dash,
                audio: audio_dash,
            }),
            ..Default::default()
        };

        Ok(result)
    }
}
