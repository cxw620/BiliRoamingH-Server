#[cfg(feature = "full")]
pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
/// Unified error type for lib-rpc-client
pub enum Error {
    #[error(transparent)]
    Hyper(#[from] hyper_014::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    GrpcStatus(#[from] tonic::Status),

    #[error("Parsing [{url}] error: {:?}", source)]
    /// Error when parsing url
    UrlParse {
        url: String,
        #[source]
        source: url::ParseError,
    },

    /// Check Response Status Error
    #[error("Invalid response with HTTP StatusCode [{0}]")]
    HttpStatus(u16),

    /// Error when parsing gRPC response
    #[error("Error when parsing gRPC response: not Grpc Response")]
    NotGrpcResponse,

    /// Received unknwon data struct when parsing data
    #[error("Received unknown data struct")]
    UnknownDataStruct,

    #[error(transparent)]
    ProxyError(#[from] ProxyError),

    /// BiliError passed through
    #[error(transparent)]
    BiliError(#[from] lib_utils::error::BiliError),

    /// Unknown error or private error
    #[error("Unknown error")]
    Unknown,

    /// Upstream error wrapped with `anyhow`,
    /// should downcast to actual one
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error(transparent)]
    ResolveSocksIo(#[from] std::io::Error),
    #[error(transparent)]
    InvalidUri(#[from] http_02::uri::InvalidUri),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
    #[error("Invalid proxy scheme [{0:?}]")]
    InvalidProxyScheme(Option<String>),
    #[error("Invalid proxy host [{0:?}]")]
    InvalidProxyHost(Option<String>),
}
