pub mod playurl;
pub(crate) mod client {
    pub use lib_rpc_client::client::grpc;
    pub use lib_rpc_client::client::rest;
    pub use lib_rpc_client::utils;
}
pub mod interface;

pub(crate) use lib_bilibili::bapis;
