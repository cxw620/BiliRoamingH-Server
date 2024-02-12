extern crate services;

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use lib_core::server::config::{init_config, CONFIG_SERVER};
use services::handler::{playurl::PlayurlRouter, test::RouterTest, InterceptHandler};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry().with(fmt::layer()).init();

    init_config();

    let app = axum::Router::new()
        .merge(PlayurlRouter::new())
        .nest("/test", RouterTest::new())
        .fallback::<_, ()>(InterceptHandler::default());

    let listen = CONFIG_SERVER.get().unwrap().listen;
    let listener = tokio::net::TcpListener::bind(listen).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
