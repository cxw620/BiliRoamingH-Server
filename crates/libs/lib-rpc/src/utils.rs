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
}

impl UpstreamType {
    const fn str(&self) -> &'static str {
        match self {
            Self::ApiBilibiliCom => "api.bilibili.com",
            Self::AppBilibiliCom => "app.bilibili.com",
            Self::GrpcBiliapiNet => "grpc.biliapi.net",
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
            u_custom: u_custom.map(std::borrow::Cow::Borrowed),
        }
    }

    pub fn with_custom(mut self, u_custom: &'u str) -> Self {
        self.u_custom = Some(Cow::Borrowed(u_custom));
        self
    }

    #[inline]
    pub fn str<'s: 'u>(&'s self) -> &'s str {
        match self.u_custom.as_ref() {
            Some(c) => c.as_ref(),
            None => self.u_type.str(),
        }
    }

    #[inline]
    pub fn url(&self) -> Result<Url> {
        Url::parse(self.str()).map_err(|e| anyhow!(RpcError::PreRequest(Kind::from(e))))
    }

    #[inline]
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
        }
    }
}
