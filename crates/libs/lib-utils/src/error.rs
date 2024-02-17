use axum::response::{IntoResponse, Response as AxumResponse};
use tracing::{debug, error};

use std::{borrow::Cow, error::Error as StdError};

use crate::model::response::GeneralResponse;

pub trait TError<'e>: Sized + StdError {
    fn e_code(&self) -> i64 {
        5_500_000
    }

    fn e_message(&'e self) -> Cow<'e, str> {
        Cow::Borrowed("服务器内部错误")
    }

    fn e_response(&'e self) -> AxumResponse {
        GeneralResponse::<()>::new_error(self.e_code(), self.e_message()).into_response(false)
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

    // ------------------------------------------------------------------------
    // X_4XX_XXX 资源及权限错误
    // ------------------------------------------------------------------------
    /// Access Key 无效
    #[error("access_key 无效")]
    AccessKeyInvalid = 5_401_002,
    /// 用户未登录
    #[error("用户未登录")]
    UserNotLoggedIn = 4_401_101,
    /// 账户被封停
    #[error("账户被封停")]
    AccountIsBaned = 5_401_102,
    /// 访问权限不足: 大会员专享限制
    #[error("访问权限不足: 大会员专享限制")]
    VipOnly = 3_403_101,
    /// 访问权限不足: 东南亚 Premium 专享限制
    #[error("访问权限不足: 东南亚 Premium 专享限制")]
    VipOnlySEA = 3_403_901,
    /// 播放平台限制
    ///
    /// - O ECode = `-10403`, O EMessage = `抱歉您所使用的平台不可观看！`
    #[error("抱歉您所使用的平台不可观看！")]
    PlatformLimitRpcRes = 3_404_819,
    /// 请求方法被禁止
    #[error("请求方法被禁止")]
    MethodNotAllowed = 3_405_000,
    /// 请求格式不合法: POST 内容应当为 JSON
    #[error("请求格式不合法")]
    RequestInvalidJson = 4_406_101,
    /// 非法请求被拦截: 通用
    #[error("非法请求被拦截")]
    FatalReqInvalid = 5_412_000,
    /// 非法请求被拦截: IP 非法
    #[error("非法请求被拦截")]
    FatalIpInvalid = 5_412_101,
    /// 非法请求被拦截: UA 非法
    #[error("非法请求被拦截")]
    FatalUaInvalid = 5_412_102,
    /// 非法请求被拦截: 签名非法
    #[error("非法请求被拦截: 签名非法")]
    FatalUrlSignInvalid = 5_412_103,
    /// 非法请求被拦截: 请求参数非法
    #[error("非法请求被拦截")]
    FatalReqParamInvalid = 5_412_201,
    /// 非法请求被拦截: 缺少请求参数
    #[error("非法请求被拦截: 请求参数异常")]
    FatalReqParamMissing = 5_412_202,
    /// 请求内容被锁定(-423)
    #[error("请求内容被锁定")]
    ReqContentLocked = 5_423_000,
    /// 请求过于频繁(-429)
    #[error("请求过于频繁")]
    ReqTooFrequent = 3_429_000,
    /// (因为漫游黑名单等)请求被拒绝: 漫游黑名单封禁
    #[error("漫游黑名单封禁")]
    RoamingBlacklisted = 5_451_101,
    /// (因为漫游黑名单等)请求被拒绝: 漫游白名单封禁(服务器仅供白名单用户使用)
    #[error("漫游白名单封禁: 服务器仅供白名单用户使用")]
    RoamingWhitelistedOnly = 5_451_102,
    /// (因为漫游黑名单等)请求被拒绝: 漫游功能仅限大会员使用限制
    #[error("服务器仅限大会员使用")]
    RoamingVipOnly = 5_451_201,
    #[error("漫游资源封禁: 不支持的资源")]
    RoamingContentLimit = 5_451_301,

    // ------------------------------------------------------------------------
    // X_500_XXX 服务器内部错误: 程序内部导致的错误
    // ------------------------------------------------------------------------
    #[num_enum(default)]
    /// 服务器内部错误(兜底)
    #[error("服务器内部错误")]
    General = 5_500_000,
    /// 程序序列化相关错误, 含 gRPC 序列化错误和 JSON 解析错误
    #[error("服务器内部错误: 序列化错误")]
    Serialization = 5_500_101,
    /// 数据库相关错误
    #[error("服务器内部错误: 数据库错误")]
    Database = 5_500_102,
    /// `-663` 错误: `appkey` 和 `access_key`, `mobi_app` 不对应
    #[error("服务器内部错误")]
    ReqAppkeyNotMatch = 5_500_901,
    /// `-663` 错误: 可能是此鉴权相关api已经被弃用, 或不适用于当前appkey
    #[error("服务器内部错误")]
    ReqAppkeyInvalid = 5_500_902,

    // ------------------------------------------------------------------------
    // X_501_XXX 服务器内部错误: 程序不支持的服务
    // ------------------------------------------------------------------------
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
    #[error("服务器内部错误: 哔哩漫游模式")]
    RoamingMode = 5_501_801,

    // ------------------------------------------------------------------------
    // X_502_XXX 上游服务不可用 / 异常(大部分时候不应该直接返回直接来自错误码)
    // ------------------------------------------------------------------------
    /// 请求上游时返回: 应用程序不存在或已被封禁(O ECode = -1)
    #[error("服务器内部错误: 上游服务不可用(O=-1)")]
    RpcReqApiFatal = 5_502_001,
    /// 请求上游时返回: API校验密匙错误(O ECode = -3)
    #[error("服务器内部错误: 上游服务不可用(O=-3)")]
    RpcReqApiSignInvalid = 5_502_003,
    /// 请求上游时返回: 请求错误(O ECode = -400)
    #[error("服务器内部错误: 上游服务不可用(O=-400)")]
    RpcReqInvalid = 5_502_400,
    /// 请求上游时返回: 请求未授权(O ECode = -401)
    #[error("服务器内部错误: 上游服务不可用(O=-401)")]
    RpcReqUnauthorized = 5_502_401,
    /// 请求上游时返回: 访问权限不足(O ECode = -403)
    #[error("服务器内部错误: 上游服务不可用(O=-403)")]
    RpcReqAccessDenied = 5_502_403,
    /// 请求上游时返回: 啥都木有(O ECode = -404)
    #[error("服务器内部错误: 上游服务不可用(O=-404)")]
    RpcReqNotFound = 5_502_404,
    /// 请求上游时返回: 请求被风控(O ECode = -412)
    #[error("服务器内部错误: 上游服务不可用(O=-412)")]
    RpcReqRiskControl = 5_502_412,
    /// 请求上游时: 429 Too Many Requests
    #[error("服务器内部错误: 上游服务不可用(E=429)")]
    RpcReqRateLimit = 5_502_429,
    /// 请求上游时: 网络错误
    #[error("服务器内部错误: 上游服务不可用(网络错误)")]
    RpcNetworkFatal = 5_502_444,
    /// 请求上游时: 500 Internal Server Error, or O ECode = -500
    #[error("服务器内部错误: 上游服务不可用(E=500)")]
    RpcReqServerInternal = 5_502_500,
    /// 请求上游时: 501 Not Implemented, or O ECode = -501
    #[error("服务器内部错误: 上游服务不可用(E=501)")]
    RpcReqServerNotImplemented = 5_502_501,
    /// 上游服务不可用: 502 Bad Gateway, or O ECode = -502
    #[error("服务器内部错误: 上游服务不可用(E=502)")]
    RpcReqBadGateway = 5_502_502,
    /// 请求上游时: 503 Service Unavailable, or O ECode = -503
    #[error("服务器内部错误: 上游服务不可用(E=503)")]
    RpcReqServiceUnavailable = 5_502_503,
    /// 请求上游时: 504 Gateway Timeout
    #[error("服务器内部错误: 上游服务不可用(E=504)")]
    RpcGatewayTimeout = 5_502_504,
    /// 上游服务不可用(gRPC): 上游服务不可用(gRPC): The operation was cancelled
    #[error("服务器内部错误: 上游服务不可用(gRPC Operation cancelled)")]
    GrpcReqCancelled = 5_502_901,
    /// 上游服务不可用(gRPC): 上游服务不可用(gRPC): Unknown error
    ///
    /// Should not come with such error.
    #[error("服务器内部错误: 上游服务不可用(gRPC Unknown error)")]
    GrpcReqUnknown = 5_502_902,
    /// 上游服务不可用(gRPC): Client specified an invalid argument
    #[error("服务器内部错误: 上游服务不可用(gRPC Client specified an invalid argument)")]
    GrpcReqInvalidArgument = 5_502_903,
    /// 上游服务不可用(gRPC): Deadline expired before operation could complete
    #[error("服务器内部错误: 上游服务不可用(gRPC Operation deadline exceeded)")]
    GrpcReqDeadlineExceeded = 5_502_904,
    /// 上游服务不可用(gRPC): Some requested entity was not found
    #[error("服务器内部错误: 上游服务不可用(gRPC Requested entity not found)")]
    GrpcReqNotFound = 5_502_905,
    /// 上游服务不可用(gRPC): Some entity that we attempted to create already exists
    #[error("服务器内部错误: 上游服务不可用(gRPC Requested creation already exists)")]
    GrpcReqAlreadyExists = 5_502_906,
    /// 上游服务不可用(gRPC): The caller does not have permission to execute the specified operation
    #[error("服务器内部错误: 上游服务不可用(gRPC Operation permission denied)")]
    GrpcReqPermissionDenied = 5_502_907,
    /// 上游服务不可用(gRPC): Some resource has been exhausted
    #[error("服务器内部错误: 上游服务不可用(gRPC Resource exhausted)")]
    GrpcReqResourceExhausted = 5_502_908,
    /// 上游服务不可用(gRPC): The system is not in a state required for the operation's execution
    #[error("服务器内部错误: 上游服务不可用(gRPC Operation failed precondition)")]
    GrpcReqFailedPrecondition = 5_502_909,
    /// 上游服务不可用(gRPC): The operation was aborted
    #[error("服务器内部错误: 上游服务不可用(gRPC Operation aborted)")]
    GrpcReqAborted = 5_502_910,
    /// 上游服务不可用(gRPC): Operation was attempted past the valid range
    #[error("服务器内部错误: 上游服务不可用(gRPC Operation past the valid range)")]
    GrpcReqOutOfRange = 5_502_911,
    /// 上游服务不可用(gRPC): Operation is not implemented or not supported
    #[error("服务器内部错误: 上游服务不可用(gRPC Operation not implemented or supported)")]
    GrpcReqUnimplemented = 5_502_912,
    /// 上游服务不可用(gRPC): Internal error
    #[error("服务器内部错误: 上游服务不可用(gRPC Internal error)")]
    GrpcReqInternal = 5_502_913,
    /// 上游服务不可用(gRPC): The service is currently unavailable
    #[error("服务器内部错误: 上游服务不可用(gRPC Service unavailable)")]
    GrpcReqUnavailable = 5_502_914,
    /// 上游服务不可用(gRPC): Unrecoverable data loss or corruption
    #[error("服务器内部错误: 上游服务不可用(gRPC Unrecoverable data loss or corruption)")]
    GrpcReqDataLoss = 5_502_915,
    /// 上游服务不可用(gRPC): The request does not have valid authentication credential
    #[error("服务器内部错误: 上游服务不可用(gRPC Request unauthenticated)")]
    GrpcReqUnauthenticated = 5_502_916,

    // ------------------------------------------------------------------------
    // X_503_XXX 服务器不可用
    // ------------------------------------------------------------------------
    /// 服务器不可用(兜底?)
    #[error("服务器内部错误: 服务器不可用")]
    ServerFatal = 5_503_000,

    // ------------------------------------------------------------------------
    // X_54X_XXX 服务器环境因素导致的资源不可用
    //
    // ------------------------------------------------------------------------
    /// DRM 限制
    ///
    /// - DRM 限制 IP 为家宽 IP: O ECode = `-10500`, O EMessage = `处理失败`
    #[error("DRM 限制机房 IP")]
    ServerIPDrmLimit = 5_541_101,
    /// 服务器网络错误: 受区域限制
    #[error("服务器网络错误: 受区域限制")]
    ServerIPAreaLimit = 5_541_110,
    /// 服务器网络错误: 受区域限制, 无法访问仅限大陆资源
    #[error("服务器网络错误: 受区域限制")]
    ServerIPAreaLimitCN = 5_541_111,
    /// 服务器网络错误: 受区域限制, 无法访问仅限港澳台资源
    #[error("服务器网络错误: 受区域限制")]
    ServerIPAreaLimitHKMOTW = 5_541_112,
    /// 服务器网络错误: 受区域限制, 无法访问仅限港澳资源
    #[error("服务器网络错误: 受区域限制")]
    ServerIPAreaLimitHKMO = 5_541_113,
    /// 服务器网络错误: 受区域限制, 无法访问仅限台湾资源
    #[error("服务器网络错误: 受区域限制")]
    ServerIPAreaLimitTW = 5_541_114,
    /// 服务器网络错误: 受区域限制, 无法访问仅限东南亚资源
    #[error("服务器网络错误: 受区域限制")]
    ServerIPAreaLimitSEA = 5_541_115,
}

impl<'e> TError<'e> for ServerError {
    fn e_code(&self) -> i64 {
        Into::<i64>::into(*self)
    }
    fn e_message(&self) -> Cow<'e, str> {
        self.to_string().into()
    }
}

impl IntoResponse for ServerError {
    #[tracing::instrument(level = "error", name="ServerError into_response")]
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
                    error!("Unknown anyhow error: {}", &source);
                    // ! Generating backtrace is REALLY EXPENSIVE,
                    // ! DO NOT CALL IN RELEASE MODE.
                    // ! If you want only panics to have backtraces,
                    // ! set RUST_BACKTRACE=1 and RUST_LIB_BACKTRACE=0.
                    debug!(
                        "###### ANYHOW ERR ######\n{}\n###### BACK TRACE ######",
                        source.backtrace()
                    );
                    "服务器内部错误".into()
                }
            }
            Self::Custom { message, .. } => message.into(),
        }
    }
}

impl IntoResponse for ServerErrorExt {
    #[tracing::instrument(level = "error", name="ServerErrorExt into_response")]
    fn into_response(self) -> axum::response::Response {
        self.e_response()
    }
}

impl From<anyhow::Error> for ServerErrorExt {
    #[tracing::instrument(level = "error", name="ServerErrorExt from anyhow::Error")]
    fn from(e: anyhow::Error) -> Self {
        if let Some(server_error) = e.downcast_ref() {
            return Self::Server(*server_error);
        }
        if let Some(bili_error) = e.downcast_ref::<BiliError>() {
            return (bili_error.to_owned()).into();
        }
        if let Some(header_error) = e.downcast_ref::<HeaderError>() {
            // TODO: Trace error using open-telemetry here
            error!("Detect HeaderError: {:?}", header_error);
            return Self::Server(ServerError::General);
        }
        Self::Any { source: e }
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum BiliError {
    /// 可能为脏数据
    ///
    /// c = `0`
    #[error("C: OK is not error")]
    Ok,
    /// 致命错误, API被弃用
    ///
    /// c = `-1`, m = `应用程序不存在或已被封禁`
    #[error("R: 应用程序不存在或已被封禁(E=-1)")]
    ApiFatal,
    /// access_key 无效
    ///
    /// c = `-2`, m = ?
    #[error("C: access_key 无效(E=-2)")]
    AccessKeyInvalid,
    /// API校验密匙错误
    ///
    /// c = `-3`, m = `API校验密匙错误`
    #[error("R: API校验密匙错误(E=-3)")]
    ApiSignInvalid,
    /// 用户未登录
    ///
    /// c = `-101`, m = `用户未登录`
    #[error("R: 用户未登录(E=-101)")]
    AccountIsNotLogin,
    /// 使用登录状态访问了，并且登录状态无效，客服端可以／需要删除登录状态
    ///
    /// + 考虑 `access_key` 和 `appkey` 不匹配
    ///
    /// c = `61000`, m = `使用登录状态访问了，并且登录状态无效，客服端可以／需要删除登录状态`
    #[error("R: 使用登录状态访问了，并且登录状态无效，客服端可以／需要删除登录状态(E=61000)")]
    AccountIsNotLoginOauth,
    /// 账号被封停
    ///
    /// c = `-102`, m = `账号被封停`
    #[error("R: 账号被封停(E=-102)")]
    AccountIsBannded,
    /// 请求错误
    ///
    /// - 请求 Param 存在问题(缺失/错误等)
    ///
    /// c = `-400`, m = `请求错误`
    #[error("R: 请求错误(E=-400)")]
    ReqInvalid,
    /// 请求未授权
    ///
    /// - 风控?
    ///
    /// c = `-401`, m = `请求未授权`
    #[error("R: 请求未授权(E=-401)")]
    ReqUnauthorized,
    /// 访问权限不足
    ///
    /// - 风控?
    /// - WBI 签名错误?
    ///
    /// c = `-403`, m = `访问权限不足`
    #[error("R: 访问权限不足(E=-403)")]
    ReqAccessDenied,
    /// 啥都木有
    ///
    /// c = `-404`, m = `啥都木有`
    #[error("R: 啥都木有(E=-404)")]
    ReqNotFound,
    /// 请求被拦截
    ///
    /// - 风控?
    ///
    /// c = `-412`, m = `请求被拦截`
    #[error("R: 请求被拦截(E=-412)")]
    ReqRiskControl,
    /// 服务器错误
    ///
    /// c = `-500`
    #[error("C: -500")]
    ServerInternal,
    /// ???
    ///
    /// c = `-501`
    #[error("C: -501")]
    ServerSystemError,
    /// ???
    ///
    /// c = `-502`
    #[error("C: -502")]
    SearchSessionIsExists,
    /// 过载保护,服务暂不可用
    ///
    /// c = `-503`
    #[error("R: 过载保护,服务暂不可用(E=-503)")]
    ServerOverload,
    /// 服务调用超时
    ///
    /// c = `-504`
    #[error("C: -504")]
    ServerTimeout,
    /// `appkey` 和 `access_key`, `mobi_app`等不对应
    ///
    /// c = `-663`, m = `鉴权失败，请联系账号组`
    #[error("R: 鉴权失败，请联系账号组(E=-663)")]
    ReqAppkeyNotMatch,
    /// 可能是此鉴权相关api已经被弃用, 或不适用于当前appkey
    ///
    /// c = `-663`, m = `-663`
    #[error("R: -663(E=-663)")]
    ReqAppkeyInvalid,
    /// 爬虫限制
    ///
    /// c = `-1200`, m = `被降级过滤的请求`
    #[error("R: 被降级过滤的请求(E=-1200)")]
    ReqCrawlerLimit,
    /// DRM 限制 IP 为家宽
    ///
    /// c = `-10500`, m = `处理失败`
    #[error("C: DRM 限制 IP 为家宽(E=-10500)")]
    ResDrmLimit,
    /// 大会员专享限制
    ///
    /// - c = `6002105`, m = `开通大会员观看`
    /// - c = `-10403`, m = `大会员专享限制`
    #[error("C: 开通大会员观看(E=6002105)")]
    ResVipOnly,
    /// 泰区 Premium 限制
    ///
    /// c = `10015002`, m = `访问权限不足`
    #[error("C: 泰区 Premium 限制(E=10015002)")]
    ResBStarVipOnly,
    /// 资源区域限制
    ///
    /// - c = `6002003`, m = `抱歉您所在地区不可观看！`
    /// - c = `-10403`, m = `抱歉您所在地区不可观看！`
    #[error("R: 抱歉您所在地区不可观看！(E=6002003)")]
    ResAreaLimit,
    /// ???
    ///
    /// c = `6010001`, m = ?
    #[error("C: 6010001")]
    ResSeasonAreaLimit,
    /// 播放平台限制
    ///
    /// - c = `-10403`, m = `抱歉您所使用的平台不可观看！`
    #[error("C: 抱歉您所使用的平台不可观看！(E=-10403)")]
    ResPlatformLimit,
    /// 未知兜底
    #[error("Unknown E={code}, M={message}")]
    Unknown { code: i64, message: String },
}

impl TryFrom<(i64, &str)> for BiliError {
    type Error = ();
    #[tracing::instrument(level = "error", name = "BiliError", ret)]
    fn try_from(value: (i64, &str)) -> Result<Self, ()> {
        let e = match value.0 {
            -1 => Self::ApiFatal,
            -2 => Self::AccessKeyInvalid,
            -3 => Self::ApiSignInvalid,
            -101 => Self::AccountIsNotLogin,
            61000 => Self::AccountIsNotLoginOauth,
            -102 => Self::AccountIsBannded,
            -400 => Self::ReqInvalid,
            -401 => Self::ReqUnauthorized,
            -403 => Self::ReqAccessDenied,
            -404 => Self::ReqNotFound,
            -412 => Self::ReqRiskControl,
            -500 => Self::ServerInternal,
            -501 => Self::ServerSystemError,
            -502 => Self::SearchSessionIsExists,
            -503 => Self::ServerOverload,
            -504 => Self::ServerTimeout,
            -663 if value.1 == "鉴权失败，请联系账号组" => Self::ReqAppkeyNotMatch,
            -663 if value.1 == "-663" => Self::ReqAppkeyInvalid,
            -1200 => Self::ReqCrawlerLimit,
            -10500 if value.1 == "处理失败" => Self::ResDrmLimit,
            6002105 => Self::ResVipOnly,
            -10403 if value.1 == "大会员专享限制" => Self::ResVipOnly,
            10015002 => Self::ResBStarVipOnly,
            6002003 => Self::ResAreaLimit,
            -10403 if value.1 == "抱歉您所在地区不可观看！" => Self::ResAreaLimit,
            6010001 => Self::ResSeasonAreaLimit,
            -10403 if value.1 == "抱歉您所使用的平台不可观看！" => {
                Self::ResPlatformLimit
            }
            0 => return Err(()),
            _ => Self::Unknown {
                code: value.0,
                message: value.1.to_owned(),
            },
        };
        Ok(e)
    }
}

impl From<BiliError> for ServerErrorExt {
    #[tracing::instrument(level = "error", name = "ServerErrorExt from BiliError")]
    fn from(value: BiliError) -> Self {
        let server_error = match value {
            BiliError::Ok => {
                error!("BiliError::Ok is not error!");
                ServerError::General
            }
            BiliError::ApiFatal => ServerError::RpcReqApiFatal,
            BiliError::AccessKeyInvalid => ServerError::AccessKeyInvalid,
            BiliError::ApiSignInvalid => ServerError::RpcReqApiSignInvalid,
            BiliError::AccountIsNotLogin | BiliError::AccountIsNotLoginOauth => {
                ServerError::UserNotLoggedIn
            }
            BiliError::AccountIsBannded => ServerError::AccountIsBaned,
            BiliError::ReqInvalid => ServerError::RpcReqInvalid,
            BiliError::ReqUnauthorized => ServerError::RpcReqUnauthorized,
            BiliError::ReqAccessDenied => ServerError::RpcReqAccessDenied,
            BiliError::ReqNotFound => ServerError::RpcReqNotFound,
            BiliError::ReqRiskControl => ServerError::RpcReqRiskControl,
            BiliError::ServerInternal => ServerError::RpcReqServerInternal,
            BiliError::ServerSystemError => ServerError::RpcReqServerNotImplemented,
            BiliError::SearchSessionIsExists => ServerError::RpcReqBadGateway,
            BiliError::ServerOverload => ServerError::RpcReqServiceUnavailable,
            BiliError::ServerTimeout => ServerError::RpcGatewayTimeout,
            BiliError::ReqAppkeyNotMatch => ServerError::ReqAppkeyNotMatch,
            BiliError::ReqAppkeyInvalid => ServerError::ReqAppkeyInvalid,
            BiliError::ReqCrawlerLimit => ServerError::RpcReqRiskControl,
            BiliError::ResDrmLimit => ServerError::ServerIPDrmLimit,
            BiliError::ResVipOnly => ServerError::VipOnly,
            BiliError::ResBStarVipOnly => ServerError::VipOnlySEA,
            BiliError::ResAreaLimit => ServerError::ServerIPAreaLimit,
            BiliError::ResSeasonAreaLimit => ServerError::ServerIPAreaLimit,
            BiliError::ResPlatformLimit => ServerError::PlatformLimitRpcRes,
            BiliError::Unknown { code, message } => return Self::Custom { code, message },
        };
        Self::Server(server_error)
    }
}

use crate::{parse_grpc_any, str_concat};
use lib_bilibili::bapis::rpc::Status as BiliGrpcStatus;

impl From<tonic::Status> for ServerErrorExt {
    #[tracing::instrument(level = "error", name = "ServerErrorExt from tonic::Status")]
    fn from(e: tonic::Status) -> Self {
        let grpc_code = e.code();
        match grpc_code {
            tonic::Code::Unknown => {
                let details = parse_grpc_any!(e.details(), BiliGrpcStatus);
                for item in details.details.iter() {
                    if item.type_url.contains("bilibili.rpc.Status") {
                        let bili_rpc_status =
                            parse_grpc_any!(item.value.as_slice(), BiliGrpcStatus);
                        let e_code = bili_rpc_status.code as i64;
                        let e_message = bili_rpc_status.message;
                        let e_details = bili_rpc_status.details;
                        error!(
                            "gRPC Uptream Error: code={}, message={}, details={:?}",
                            e_code, e_message, e_details
                        );
                        return if let Ok(e) = BiliError::try_from((e_code, e_message.as_str())) {
                            Self::from(e)
                        } else {
                            Self::Custom {
                                code: e_code,
                                message: str_concat!(
                                    "gRPC Uptream Unknown BiliError: ",
                                    &e_message
                                ),
                            }
                        };
                    }
                }
                error!("Unknown gRPC Uptream Error: {:?}", e);
                Self::Server(ServerError::GrpcReqUnknown)
            }
            _ => Self::Server(ServerError::from(grpc_code as i64 + 5_502_900)),
        }
    }
}

// The following are internal errors, should not exposed to user.

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
/// HeaderError
///
/// **Internal error, should not exposed to user**
pub(crate) enum HeaderError {
    #[error("Key [{0}] not exist")]
    KeyNotExist(String),
    #[error("Base64DecodeError {e}: Key [{key}] => Value [{value}]")]
    Base64DecodeError {
        key: String,
        value: String,
        e: base64::DecodeError,
    },
    #[error(transparent)]
    ToStrError(#[from] http::header::ToStrError),
    #[error(transparent)]
    ToStrError02(#[from] http_02::header::ToStrError),
    #[error(transparent)]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
    #[error(transparent)]
    InvalidHeaderValue02(#[from] http_02::header::InvalidHeaderValue),
}
