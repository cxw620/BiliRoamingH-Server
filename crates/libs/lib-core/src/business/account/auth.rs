use serde::{Deserialize, Serialize};

/// 登录信息
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthInfo {
    /// 访问令牌
    pub token_info: TokenInfo,
    /// Cookie 信息
    pub cookie_info: CookieInfo,
    /// 消息
    pub message: String,
    /// 状态
    pub status: i32,
    /// URL
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenInfo {
    /// 访问令牌(or `access_key`)
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 令牌有效期至(秒时间戳)
    pub expires: i64,
    /// 令牌有效期
    pub expires_in: i64,
    /// 用户 MID
    pub mid: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CookieInfo {
    pub cookie: Vec<CookieBean>,
    pub domain: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CookieBean {
    pub expires: i64,
    pub http_only: i32,
    pub name: String,
    pub secure: i32,
    pub value: String,
}
