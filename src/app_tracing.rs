use opentelemetry;
use std::error::Error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

pub(crate) fn setup() -> Result<(), Box<dyn Error>> {
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("snapshot proxy")
        .install_batch(opentelemetry::runtime::Tokio)?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(
            HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true),
        )
        .with(telemetry)
        .init();

    Ok(())
}

pub(crate) fn teardown() {
    opentelemetry::global::shutdown_tracer_provider();
}
