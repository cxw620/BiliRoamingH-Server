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
    (GET => $handler:expr) => {
        axum::routing::MethodRouter::new().get::<_, ()>($handler)
    };
    (POST => $handler:expr) => {
        axum::routing::MethodRouter::new().post::<_, ()>($handler)
    };
    (PUT => $handler:expr) => {
        axum::routing::MethodRouter::new().put::<_, ()>($handler)
    };
    (DELETE => $handler:expr) => {
        axum::routing::MethodRouter::new().delete::<_, ()>($handler)
    };
    (PATCH => $handler:expr) => {
        axum::routing::MethodRouter::new().patch::<_, ()>($handler)
    };
    (OPTIONS => $handler:expr) => {
        axum::routing::MethodRouter::new().options::<_, ()>($handler)
    };
    (HEAD => $handler:expr) => {
        axum::routing::MethodRouter::new().head::<_, ()>($handler)
    };
    (CONNECT => $handler:expr) => {
        axum::routing::MethodRouter::new().connect::<_, ()>($handler)
    };
}

#[macro_export]
macro_rules! generate_router {
    ($router_name:ident, $( ($route:expr, $method:ident, $handler:expr) ),*) => {
        pub struct $router_name;

        impl $router_name {
            pub fn new() -> axum::Router {
                let mut router = axum::Router::new();

                $(
                    router = router.route(
                        $route,
                        crate::axum_route!($method => $handler),
                    );
                )*

                router
            }
        }
    };
}

/// Convert headers between from http 0.2 and http 1.0
#[macro_export]
macro_rules! http02_compat {
    ($name:ident, $original_map:expr, $compat:ident) => {
        let mut $name = $compat::HeaderMap::with_capacity($original_map.len());
        {
            let mut key = None;
            $original_map.into_iter().for_each(|(k, v)| {
                if let Some(k) = k {
                    key = Some(k)
                }

                let key = key.as_ref().unwrap().as_str().as_bytes();
                let k = $compat::HeaderName::from_bytes(key).unwrap();
                let v = $compat::HeaderValue::from_bytes(v.as_bytes()).unwrap();

                $name.append(k, v);
            })
        }
    };
}
