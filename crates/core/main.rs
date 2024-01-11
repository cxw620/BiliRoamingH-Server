extern crate services;

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use services::{DefaultHandler, RouterTest, playurl::PlayurlRouter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry().with(fmt::layer()).init();

    let app = axum::Router::new()
        .merge(PlayurlRouter::new())
        .nest("/test", RouterTest::new())
        .route("/favicon.ico", axum::routing::any(|| async { axum::http::StatusCode::NOT_FOUND }))
        .fallback::<_, ()>(DefaultHandler);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:2663").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
