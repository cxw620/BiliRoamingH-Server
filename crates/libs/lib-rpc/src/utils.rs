use anyhow::{anyhow, Result};
use http_02::Uri;
use url::Url;

use std::borrow::Cow;

use crate::error::{Kind, RpcError};

#[cfg(feature = "request")]
pub(crate) use lib_utils::misc::BiliArea;

#[derive(Debug, Clone)]
pub enum UpstreamType {
    ApiBilibiliCom,
    AppBilibiliCom,
    GrpcBiliapiNet,
    Custom,
}

impl UpstreamType {
    const fn str(&self) -> &'static str {
        match self {
            Self::ApiBilibiliCom => "https://api.bilibili.com",
            Self::AppBilibiliCom => "https://app.bilibili.com",
            Self::GrpcBiliapiNet => "https://grpc.biliapi.net",
            Self::Custom => panic!("Custom upstream type has no default value."),
        }
    }

    pub fn upstream(self) -> Upstream<'static> {
        Upstream::from(self)
    }
}

#[derive(Debug, Clone)]
pub struct Upstream<'u> {
    pub u_type: UpstreamType,
    pub u_custom: Option<std::borrow::Cow<'u, str>>,
}

impl<'u> Upstream<'u> {
    pub const API_DEFAULT: Upstream<'static> = Upstream {
        u_type: UpstreamType::ApiBilibiliCom,
        u_custom: None,
    };
    pub const APP_DEFAULT: Upstream<'static> = Upstream {
        u_type: UpstreamType::AppBilibiliCom,
        u_custom: None,
    };
    pub const GRPC_DEFAULT: Upstream<'static> = Upstream {
        u_type: UpstreamType::GrpcBiliapiNet,
        u_custom: None,
    };

    #[inline]
    pub fn new(u_type: UpstreamType, u_custom: Option<&'u str>) -> Self {
        Self {
            u_type,
            u_custom: u_custom.map(Cow::Borrowed),
        }
    }

    #[inline]
    /// Create a new custom upstream.
    /// 
    /// Attention: The custom upstream must start with `https://` or `http://`.
    pub const fn new_custom(u_custom: &'u str) -> Self {
        Self {
            u_type: UpstreamType::Custom,
            u_custom: Some(Cow::Borrowed(u_custom)),
        }
    }

    #[inline]
    #[tracing::instrument(level = "debug", name = "Upstream url", err)]
    pub fn with_custom(mut self, u_custom: &'u str) -> Result<Self> {
        if u_custom.starts_with("https://") || u_custom.starts_with("http://") {
            self.u_custom = Some(Cow::Borrowed(u_custom));
        } else {
            return Err(anyhow!(RpcError::PreRequest(Kind::Any(anyhow!(
                "Invalid custom upstream scheme."
            )))));
        }

        Ok(self)
    }

    #[inline]
    pub fn str<'s: 'u>(&'s self) -> &'s str {
        match self.u_custom.as_ref() {
            Some(c) => c.as_ref(),
            None => self.u_type.str(),
        }
    }

    #[inline]
    #[tracing::instrument(level = "debug", name = "Upstream.url", err)]
    pub fn url(&self) -> Result<Url> {
        Url::parse(self.str()).map_err(|e| anyhow!(RpcError::PreRequest(Kind::from(e))))
    }

    #[inline]
    #[tracing::instrument(level = "debug", name = "Upstream.uri", err)]
    pub fn uri(&self) -> Result<Uri> {
        match &self.u_custom {
            Some(c) => {
                Uri::try_from(c.as_ref()).map_err(|e| anyhow!(RpcError::PreRequest(Kind::from(e))))
            }
            None => Ok(Uri::from_static(self.u_type.str())),
        }
    }
}

impl From<UpstreamType> for Upstream<'_> {
    fn from(u_type: UpstreamType) -> Self {
        match u_type {
            UpstreamType::ApiBilibiliCom => Self::API_DEFAULT,
            UpstreamType::AppBilibiliCom => Self::APP_DEFAULT,
            UpstreamType::GrpcBiliapiNet => Self::GRPC_DEFAULT,
            UpstreamType::Custom => panic!("Custom upstream type has no default value."),
        }
    }
}

impl From<String> for Upstream<'_> {
    fn from(u_custom: String) -> Self {
        Self {
            u_type: UpstreamType::Custom,
            u_custom: Some(Cow::Owned(u_custom)),
        }
    }
}

impl<'r> From<&'r str> for Upstream<'r> {
    fn from(u_custom: &'r str) -> Self {
        Self {
            u_type: UpstreamType::Custom,
            u_custom: Some(Cow::Borrowed(u_custom)),
        }
    }
}

impl<'r> From<Cow<'r, str>> for Upstream<'r> {
    fn from(u_custom: Cow<'r, str>) -> Self {
        Self {
            u_type: UpstreamType::Custom,
            u_custom: Some(u_custom),
        }
    }
}
