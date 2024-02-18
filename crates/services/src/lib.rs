pub mod handler;
pub mod intercept;
mod model;

pub type HandlerFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = axum::response::Response> + Send>>;

#[macro_export]
macro_rules! axum_response {
    ($result:expr) => {
        lib_utils::model::response::GeneralResponse::from($result).into_response(false)
    };
    ($result:expr, $data_only:expr) => {
        lib_utils::model::response::GeneralResponse::from($result).into_response($data_only)
    };
}

#[macro_export]
macro_rules! axum_route {
    (GET => $handler:path) => {
        axum::routing::MethodRouter::new().get::<_, ()>($handler)
    };
    (POST => $handler:path) => {
        axum::routing::MethodRouter::new().post::<_, ()>($handler)
    };
    (PUT => $handler:path) => {
        axum::routing::MethodRouter::new().put::<_, ()>($handler)
    };
    (DELETE => $handler:path) => {
        axum::routing::MethodRouter::new().delete::<_, ()>($handler)
    };
    (PATCH => $handler:path) => {
        axum::routing::MethodRouter::new().patch::<_, ()>($handler)
    };
    (OPTIONS => $handler:path) => {
        axum::routing::MethodRouter::new().options::<_, ()>($handler)
    };
    (HEAD => $handler:path) => {
        axum::routing::MethodRouter::new().head::<_, ()>($handler)
    };
    (CONNECT => $handler:path) => {
        axum::routing::MethodRouter::new().connect::<_, ()>($handler)
    };
}

#[macro_export]
macro_rules! generate_router {
    ($router_name:ident, $( ($route:expr, $method:ident, $handler:path) ),*) => {
        pub struct $router_name;

        impl $router_name {
            pub fn new() -> axum::Router {
                let mut router = axum::Router::new();

                $(
                    router = router.route(
                        $route,
                        axum_route!($method => $handler),
                    );
                )*

                router
            }
        }
    };
}
