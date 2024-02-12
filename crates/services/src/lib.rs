pub mod handler;
pub mod intercept;
mod model;

pub type HandlerFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = axum::response::Response> + Send>>;

#[macro_export]
macro_rules! axum_response {
    ($result:expr) => {
        crate::model::GeneralResponse::new_from_result($result).into_response(false)
    };
    ($result:expr, $data_only:expr) => {
        crate::model::GeneralResponse::new_from_result($result).into_response($data_only)
    };
}
