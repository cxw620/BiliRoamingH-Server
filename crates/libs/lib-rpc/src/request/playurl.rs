use std::borrow::Cow;

/// Playurl Related Request info
pub struct PlayurlReq<'q> {
    pub vod: VideoVod,
    pub vod_ext: VideoVodExt<'q>,
}

/// Video VOD Info
pub struct VideoVod {
    /// 视频aid
    pub aid: i32,
    /// 视频cid
    pub cid: i32,
    /// 清晰度
    pub qn: u64,
    /// 视频流版本
    pub fnver: i32,
    /// 视频流格式
    pub fnval: i32,
    /// 下载模式
    /// 0:播放 1:flv下载 2:dash下载
    pub download: u32,
    /// 流url强制是用域名
    /// 0:允许使用ip 1:使用http 2:使用https
    pub force_host: i32,
    /// 是否4K
    pub fourk: bool,
    /// 视频编码
    pub prefer_codec_type: CodeType,
    /// 响度均衡
    pub voice_balance: u64,
}

/// 视频编码
pub enum CodeType {
    /// 不指定
    Nocode = 0,
    /// H264
    Code264 = 1,
    /// H265
    Code265 = 2,
    /// AV1
    Codeav1 = 3,
}

pub struct VideoVodExt<'v> {
    pub bvid: Cow<'v, str>,
    pub vod_type: VodType,
}

pub enum VodType {
    Ugc,
    Pgc { ep_id: i64, season_id: i64 },
}
