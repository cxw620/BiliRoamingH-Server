pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
/// Unified error type for lib-rpc-client
pub enum Error {
    #[error(transparent)]
    Reqwest {
        #[from]
        source: reqwest::Error,
    },

    /// Check Response Status Error
    #[error("Invalid response with HTTP StatusCode [{0}]")]
    HttpStatus(u16),

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
