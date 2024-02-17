pub mod error;
pub mod utils;

pub(crate) use error::Error as CrateError;
pub(crate) use lib_utils::error::BiliError;

#[cfg(feature = "full")]
pub mod client {
    pub mod grpc;
    pub mod rest;
}
