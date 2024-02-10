pub mod grpc;
pub mod rest;
pub mod utils;
pub mod error;

pub(crate) use error::Error as CrateError;
pub(crate) use lib_utils::error::BiliError;