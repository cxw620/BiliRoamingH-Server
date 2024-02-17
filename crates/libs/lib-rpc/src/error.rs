use lib_rpc_client::error::Error as ClientError;

#[derive(Debug, thiserror::Error)]
pub enum Kind {
    #[error(transparent)]
    ClientError(#[from] ClientError),

    #[error(transparent)]
    UrlParse(#[from] url::ParseError),

    #[error(transparent)]
    InvalidUri(#[from] http_02::uri::InvalidUri),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum RpcError {
    /// Error occurred when preparing request
    PreRequest(Kind),
    /// Error occurred when sending request
    Request(Kind),
    /// Error occurred when processing response
    Response(Kind),
    /// Any other error
    Any(#[from] anyhow::Error),
}
