// Compatibility for Tonic using crate http@0.2
pub(super) mod connect_http02;
// Compatibility for Tonic using crate http@0.2
pub mod client_http02;
pub mod proxy;
#[cfg(feature = "__tls")]
mod tls;

// Re-export tonic::codec::CompressionEncoding
pub use tonic::codec::CompressionEncoding;
