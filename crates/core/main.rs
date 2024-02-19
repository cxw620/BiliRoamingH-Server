extern crate services;

use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use lib_core::server::config::{init_config, CONFIG_SERVER};
use services::handler::{
    playurl::PlayurlRouter, test::RouterTest, test_intercept::TestInterceptRouter, InterceptHandler,
};

#[tokio::main]
async fn main() {
    init_tracing();

    tracing::info!("Starting...");

    init_env();

    init_config();

    if cfg!(test) || cfg!(debug_assertions) {
        tracing::warn!("Running in test/debug mode, will IGNORE invalid certificates!!! For safety, please run in release mode.")
    }

    let app = axum::Router::new()
        .merge(PlayurlRouter::new())
        .merge(TestInterceptRouter::new())
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

    let tracing_filter = EnvFilter::default()
        .add_directive("otel::tracing=trace".parse().unwrap())
        .add_directive("lib_bilibili=debug".parse().unwrap())
        .add_directive("lib_core=debug".parse().unwrap())
        .add_directive("lib_rpc=debug".parse().unwrap())
        .add_directive("lib_rpc_client=debug".parse().unwrap())
        .add_directive("lib_utils=debug".parse().unwrap())
        .add_directive("services=debug".parse().unwrap())
        .add_directive("biliroamingh_rust_server=debug".parse().unwrap());

    let tracing_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_filter(tracing_filter);

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap()
        .add_directive("hyper=error".parse().unwrap());

    tracing_subscriber::registry()
        .with(tracing_layer)
        .with(fmt::layer().with_filter(filter))
        .init();
}

#[tracing::instrument]
fn init_env() {
    if let Err(e) = dotenvy::dotenv() {
        tracing::error!("Failed to load .env file: {}", e);
    };
}
