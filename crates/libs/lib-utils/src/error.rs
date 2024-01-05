use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use tracing::error;

#[derive(Serialize, Deserialize, Clone)]
struct ErrorResponse {
    pub code: i64,
    pub message: String,
}

pub trait TError<'e>: Sized + std::error::Error {
    fn e_code(&self) -> i64 {
        5_500_000
    }
    fn e_message(&'e self) -> std::borrow::Cow<'e, str> {
        Cow::Borrowed("服务器内部错误")
    }
    // fn e_json(&self) -> String {
    //     r#"{{"code": 5500000, "message": "服务器内部错误"}}"#.to_owned()
    // }
    fn e_response(&'e self) -> axum::response::Response {
        axum::response::Json(ErrorResponse {
            code: self.e_code(),
            message: self.e_message().to_string(),
        })
        .into_response()
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    num_enum::FromPrimitive,
    num_enum::IntoPrimitive,
    thiserror::Error,
)]
#[repr(i64)]
pub enum ServerError {
    /// OK 不是错误, 占位
    #[error("OK")]
    Ok = 0,
    /// 用户未登录
    #[error("用户未登录")]
    UserNotLoggedIn = 3_401_101,
    /// 账户被封停
    #[error("账户被封停")]
    AccountIsBaned = 4_401_102,
    /// 访问权限不足: 大会员专享限制
    #[error("访问权限不足: 大会员专享限制")]
    VipOnly = 2_403_101,
    /// 访问权限不足: 东南亚 Premium 专享限制
    #[error("访问权限不足: 东南亚 Premium 专享限制")]
    VipOnlySEA = 2_403_901,
    /// 区域限制资源
    #[error("区域限制")]
    AreaLimit = 3_404_900,
    /// 区域限制资源: 仅限大陆资源
    #[error("区域限制: 仅限大陆资源")]
    AreaLimitCN = 3_404_901,
    /// 区域限制资源: 仅限港澳台资源
    #[error("区域限制: 仅限港澳台资源")]
    AreaLimitHKMOTW = 3_404_902,
    /// 区域限制资源: 仅限港澳资源
    #[error("区域限制: 仅限港澳资源")]
    AreaLimitHKMO = 3_404_903,
    /// 区域限制资源: 仅限台湾资源
    #[error("区域限制: 仅限台湾资源")]
    AreaLimitTW = 3_404_904,
    /// 区域限制资源: 仅限东南亚资源
    #[error("区域限制: 仅限东南亚资源")]
    AreaLimitSEA = 3_404_905,
    /// 请求方法被禁止
    #[error("请求方法被禁止")]
    MethodNotAllowed = 4_405_000,
    /// 请求格式不合法: POST 内容应当为 JSON
    #[error("请求格式不合法")]
    RequestInvalidJson = 4_406_101,
    /// 非法请求被拦截: 通用
    #[error("非法请求被拦截")]
    FatalReqInvalid = 4_412_000,
    /// 非法请求被拦截: IP 非法
    #[error("非法请求被拦截")]
    FatalIpInvalid = 4_412_101,
    /// 非法请求被拦截: UA 非法
    #[error("非法请求被拦截")]
    FatalUaInvalid = 4_412_102,
    /// 非法请求被拦截: 签名非法
    #[error("非法请求被拦截: 签名非法")]
    FatalUrlSignInvalid = 4_412_103,
    /// 非法请求被拦截: 请求参数非法
    #[error("非法请求被拦截")]
    FatalReqParamInvalid = 4_412_201,
    /// 非法请求被拦截: 缺少请求参数
    #[error("非法请求被拦截: 缺少请求参数")]
    FatalReqParamMissing = 4_412_202,
    /// 请求内容被锁定(-423)
    #[error("请求内容被锁定")]
    ReqContentLocked = 4_423_000,
    /// 请求过于频繁(-429)
    #[error("请求过于频繁")]
    ReqTooFrequent = 4_429_000,
    /// (因为漫游黑名单等)请求被拒绝: 漫游黑名单封禁
    #[error("漫游黑名单封禁")]
    RoamingBlacklisted = 4_451_101,
    /// (因为漫游黑名单等)请求被拒绝: 漫游白名单封禁(服务器仅供白名单用户使用)
    #[error("漫游白名单封禁: 服务器仅供白名单用户使用")]
    RoamingWhitelistedOnly = 4_451_102,
    /// (因为漫游黑名单等)请求被拒绝: 漫游功能仅限大会员使用限制
    #[error("服务器仅限大会员使用")]
    RoamingVipOnly = 4_451_201,
    #[error("漫游资源封禁: 不支持的资源")]
    RoamingContentLimit = 4_451_301,
    #[num_enum(default)]
    /// 服务器内部错误(兜底)
    #[error("服务器内部错误")]
    General = 5_500_000,
    /// 序列化相关错误, 含gRPC序列化错误和JSON解析错误
    #[error("服务器内部错误: 序列化错误")]
    Serialization = 5_500_101,
    /// 不支持的服务: 弃用的 API
    #[error("服务器内部错误: API 已弃用")]
    ServicesDeprecated = 5_501_101,
    /// 不支持的服务: 不兼容/识别的 RESTful / gRPC 请求
    #[error("服务器内部错误: 不支持的服务")]
    ServicesUnsupported = 5_501_201,
    /// 未实现的服务: 未实现的 API
    #[error("服务器内部错误: 未实现的服务")]
    ServerInternalNotImpl = 5_501_301,
    /// 漫游模式不支持非目标 API
    #[error("服务器内部错误: 漫游模式")]
    RoamingMode = 5_501_901,
    /// 上游服务不可用: 网络错误
    #[error("服务器内部错误: 上游服务不可用(网络错误)")]
    NetworkFatal = 5_502_001,
    /// 上游429速率限制
    #[error("服务器内部错误: 上游服务不可用(速率限制)")]
    NetworkRateLimit = 5_502_429,
    /// 上游服务不可用: 请求超时
    #[error("服务器内部错误: 上游服务不可用(网络错误)")]
    NetworkTimeout = 5_502_504,
    /// 服务器不可用
    #[error("服务器内部错误: 服务器不可用")]
    ServerFatal = 5_503_000,
}

impl<'e> TError<'e> for ServerError {
    fn e_code(&self) -> i64 {
        // adjust to existing Bilibili style e_code
        if cfg!(feature = "roaming_mode") {
            match self {
                Self::FatalReqInvalid => -412,
                Self::FatalIpInvalid => -412,
                Self::FatalUaInvalid => -412,
                Self::FatalUrlSignInvalid => -3,
                Self::FatalReqParamInvalid => -412,
                Self::FatalReqParamMissing => -412,
                Self::ReqContentLocked => -423,
                Self::ReqTooFrequent => -429,
                _ => Into::<i64>::into(*self),
            }
        } else {
            Into::<i64>::into(*self)
        }
    }
    fn e_message(&self) -> Cow<'e, str> {
        self.to_string().into()
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        self.e_response()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServerErrorExt {
    #[error(transparent)]
    Server(#[from] ServerError),
    #[error(transparent)]
    Any { source: anyhow::Error },
    #[error("{message}")]
    Custom { code: i64, message: String },
}

impl<'e> TError<'e> for ServerErrorExt {
    fn e_code(&self) -> i64 {
        match self {
            Self::Server(e) => e.e_code(),
            Self::Any { source } => {
                if let Some(e) = source.downcast_ref::<Self>() {
                    e.e_code()
                } else if let Some(e) = source.downcast_ref::<ServerError>() {
                    e.e_code()
                } else {
                    5_500_000
                }
            }
            Self::Custom { code, .. } => *code,
        }
    }
    fn e_message(&'e self) -> Cow<'e, str> {
        match self {
            Self::Server(e) => e.e_message(),
            Self::Any { source } => {
                if let Some(e) = source.downcast_ref::<Self>() {
                    e.e_message()
                } else if let Some(e) = source.downcast_ref::<ServerError>() {
                    e.e_message()
                } else {
                    error!("Unknown anyhow error: {:?}", source);
                    "服务器内部错误".into()
                }
            }
            Self::Custom { message, .. } => message.into(),
        }
    }
}

impl IntoResponse for ServerErrorExt {
    fn into_response(self) -> axum::response::Response {
        self.e_response()
    }
}

impl From<anyhow::Error> for ServerErrorExt {
    fn from(e: anyhow::Error) -> Self {
        let err = match e.downcast::<ServerError>() {
            Ok(err) => return Self::Server(err),
            Err(err) => err,
        };

        Self::Any { source: err }
    }
}
