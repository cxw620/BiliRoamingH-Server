use anyhow::{anyhow, Result};
use http_02::{uri::Authority, HeaderValue};
use percent_encoding::percent_decode;
use std::{borrow::Cow, net::SocketAddr};
use url::Url;

use crate::error::ProxyError;
use lib_utils::{b64_encode, str_concat};

#[derive(Clone, Debug)]
pub struct Proxy {
    scheme: ProxyScheme,
    // TODO Use deadpool to maintain proxies?
    /// Whether the proxy is available last time we used or check
    available: bool,
}

#[allow(dead_code)]
impl Proxy {
    /// Create a new [`Proxy`] from given proxy url
    pub fn new(url: &str) -> Result<Self> {
        let scheme = ProxyScheme::parse(url)?;
        Ok(Self {
            scheme,
            available: true,
        })
    }

    #[inline]
    pub fn is_available(&self) -> bool {
        self.available
    }

    #[inline]
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }

    #[inline]
    pub fn scheme(&self) -> &ProxyScheme {
        &self.scheme
    }

    #[inline]
    pub fn scheme_owned(&self) -> ProxyScheme {
        self.scheme.clone()
    }
}

#[derive(Clone, Debug)]
pub enum ProxyScheme {
    Http {
        auth: Option<HeaderValue>,
        host: Authority,
    },
    Https {
        auth: Option<HeaderValue>,
        host: Authority,
    },
    #[cfg(feature = "socks")]
    Socks5 {
        addr: SocketAddr,
        auth: Option<(String, String)>,
        remote_dns: bool,
    },
}

impl ProxyScheme {
    #[tracing::instrument]
    fn parse(url: &str) -> Result<Self> {
        let url = Url::parse(url).map_err(|e| ProxyError::from(e))?;

        // Resolve URL to a host and port
        #[cfg(feature = "socks")]
        let to_addr = || {
            let addrs = url
                .socket_addrs(|| match url.scheme() {
                    "socks5" | "socks5h" => Some(7890),
                    _ => None,
                })
                .map_err(|e| ProxyError::from(e))?;
            addrs
                .into_iter()
                .next()
                .ok_or_else(|| ProxyError::InvalidProxyHost(url.host().map(|h| h.to_string())))
        };

        let auth_info = if let Some(pwd) = url.password() {
            let decoded_username = percent_decode(url.username().as_bytes()).decode_utf8_lossy();
            let decoded_password = percent_decode(pwd.as_bytes()).decode_utf8_lossy();
            Some((decoded_username, decoded_password))
        } else {
            None
        };

        use url::Position;
        Ok(match url.scheme() {
            "http" => Self::Http {
                auth: encode_basic_auth(auth_info),
                host: (&url[Position::BeforeHost..Position::AfterPort])
                    .parse()
                    .map_err(|e| ProxyError::from(e))?,
            },
            "https" => Self::Https {
                auth: encode_basic_auth(auth_info),
                host: (&url[Position::BeforeHost..Position::AfterPort])
                    .parse()
                    .map_err(|e| ProxyError::from(e))?,
            },
            #[cfg(feature = "socks")]
            "socks5" => Self::Socks5 {
                addr: to_addr()?,
                auth: auth_info.map(|(u, p)| (u.to_string(), p.to_string())),
                remote_dns: false,
            },
            #[cfg(feature = "socks")]
            "socks5h" => Self::Socks5 {
                addr: to_addr()?,
                auth: auth_info.map(|(u, p)| (u.to_string(), p.to_string())),
                remote_dns: true,
            },
            _ => {
                return Err(anyhow!(ProxyError::InvalidProxyScheme(Some(
                    url.scheme().to_string()
                ))))
            }
        })
    }
}

pub(crate) fn encode_basic_auth(
    auth_info: Option<(Cow<'_, str>, Cow<'_, str>)>,
) -> Option<HeaderValue> {
    let auth = auth_info?;
    let mut authorization = HeaderValue::from_str(&str_concat!(
        "Basic ",
        &b64_encode!(str_concat!(&auth.0, ":", &auth.1))
    ))
    .unwrap();
    authorization.set_sensitive(true);
    Some(authorization)
}
