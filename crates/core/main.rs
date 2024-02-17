extern crate services;

use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use lib_core::server::config::{init_config, CONFIG_SERVER};
use services::handler::{playurl::PlayurlRouter, test::RouterTest, InterceptHandler};

#[tokio::main]
async fn main() {
    init_tracing();

    tracing::info!("Starting...");

    init_config();

    let app = axum::Router::new()
        .merge(PlayurlRouter::new())
        .nest("/test", RouterTest::new())
        .fallback::<_, ()>(InterceptHandler::default())
        .layer(OtelInResponseLayer::default())
        .layer(OtelAxumLayer::default());

    let listen = CONFIG_SERVER.get().unwrap().listen;
    let listener = tokio::net::TcpListener::bind(listen).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    opentelemetry::global::shutdown_tracer_provider();
}

#[tracing::instrument]
fn init_tracing() {
    // Init global text map propagator
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_jaeger::Propagator::with_custom_header_and_baggage(
            "x-roamingh-trace-id",
            "x-roamingh-ctx-",
        ),
    );

    // Init Jaeger tracer
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("BiliRoamingH-Server")
        .with_endpoint("127.0.0.1:6831")
        .with_instrumentation_library_tags(false)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap();

    let tracing_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(tracing_layer)
        .with(fmt::layer())
        .init();
}
