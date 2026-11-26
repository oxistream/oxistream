use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use tracing_subscriber::EnvFilter;

use crate::config::LogFormat;

pub fn init_tracing(filter: &str, format: LogFormat) -> anyhow::Result<()> {
    let filter = EnvFilter::try_new(filter)
        .map_err(|err| anyhow::anyhow!("OXISTREAM_LOG is not a valid filter: {err}"))?;
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match format {
        LogFormat::Pretty => builder.try_init(),
        LogFormat::Json => builder.json().flatten_event(true).try_init(),
    }
    .map_err(|err| anyhow::anyhow!(err))
}

const DURATION_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

fn metrics_builder() -> PrometheusBuilder {
    PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_request_duration_seconds".into()),
            DURATION_BUCKETS,
        )
        .expect("bucket list is not empty")
}

pub fn install_metrics_recorder() -> anyhow::Result<PrometheusHandle> {
    Ok(metrics_builder().install_recorder()?)
}

/// A handle backed by a recorder that is not installed globally, for tests.
pub fn detached_metrics_handle() -> PrometheusHandle {
    metrics_builder().build_recorder().handle()
}

pub async fn track_http(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| "<unmatched>".to_owned(), |path| path.as_str().to_owned());

    let response = next.run(request).await;

    let labels = [
        ("method", method),
        ("path", path),
        ("status", response.status().as_u16().to_string()),
    ];
    metrics::counter!("http_requests_total", &labels).increment(1);
    metrics::histogram!("http_request_duration_seconds", &labels)
        .record(start.elapsed().as_secs_f64());

    response
}
