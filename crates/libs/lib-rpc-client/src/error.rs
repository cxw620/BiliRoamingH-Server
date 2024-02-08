use anyhow::anyhow;

pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("ERROR: Operation timed out")]
    TimedOut,
    #[error("ERROR HTTP Status code: {0:?}")]
    HttpStatus(u16),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("General: {0:?}")]
    General(#[from] anyhow::Error),
}

// impl From<BoxError> for ClientError {
//     fn from(e: BoxError) -> Self {
//         if let Some(e) = e.downcast_ref::<ClientError>() {
//             return
//         };

//         Self::General(anyhow!("Unknown client error"))
//     }
// }
