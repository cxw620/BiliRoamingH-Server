extern crate services;

use services::DefaultHandler;

#[tokio::main]
async fn main() {
    let app = axum::Router::new().fallback::<_, ()>(DefaultHandler);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:2663").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
