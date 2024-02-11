pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
/// Unified error type for lib-rpc-client
pub enum Error {
    #[error(transparent)]
    Hyper(#[from] hyper_014::Error),

    #[error(transparent)]
    Reqwest {
        #[from]
        source: reqwest::Error,
    },

    #[error(transparent)]
    GrpcStatus(#[from] tonic::Status),

    #[error(transparent)]
    /// Error when parsing url
    UrlParse {
        #[from]
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

    /// BiliError passed through
    #[error(transparent)]
    BiliError(#[from] lib_utils::error::BiliError),

    /// General Error type for overall error handling
    #[error(transparent)]
    General(#[from] anyhow::Error),
}

impl Into<tonic::Status> for Error {
    fn into(self) -> tonic::Status {
        match self {
            Error::GrpcStatus(status) => status,
            _ => tonic::Status::internal(self.to_string()),
        }
    }
}
