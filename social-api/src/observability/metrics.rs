use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{exponential_buckets, Histogram};
use prometheus_client::registry::Registry;
use std::sync::Arc;

pub struct AppMetrics {
    pub registry: Registry,
    pub http_requests_total: Family<HttpLabels, Counter>,
    pub http_request_duration: Family<HttpDurationLabels, Histogram>,
    #[allow(dead_code)]
    pub cache_operations_total: Family<CacheLabels, Counter>,
    #[allow(dead_code)]
    pub external_calls_total: Family<ExternalCallLabels, Counter>,
    #[allow(dead_code)]
    pub external_call_duration: Family<ExternalDurationLabels, Histogram>,
    #[allow(dead_code)]
    pub circuit_breaker_state: Family<ServiceLabel, Gauge>,
    #[allow(dead_code)]
    pub db_pool_connections: Family<PoolStateLabels, Gauge>,
    #[allow(dead_code)]
    pub sse_connections_active: Gauge,
    #[allow(dead_code)]
    pub likes_total: Family<LikeLabels, Counter>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct HttpLabels {
    pub method: String,
    pub path: String,
    pub status: u16,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct HttpDurationLabels {
    pub method: String,
    pub path: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct CacheLabels {
    pub operation: String,
    pub result: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct ExternalCallLabels {
    pub service: String,
    pub method: String,
    pub status: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct ExternalDurationLabels {
    pub service: String,
    pub method: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct ServiceLabel {
    pub service: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct PoolStateLabels {
    pub state: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, prometheus_client::encoding::EncodeLabelSet)]
pub struct LikeLabels {
    pub content_type: String,
    pub operation: String,
}

impl AppMetrics {
    pub fn new() -> Arc<Self> {
        let mut registry = Registry::default();

        let http_requests_total = Family::default();
        registry.register(
            "social_api_http_requests_total",
            "Total HTTP requests",
            http_requests_total.clone(),
        );

        let http_request_duration =
            Family::<HttpDurationLabels, Histogram>::new_with_constructor(|| {
                Histogram::new(exponential_buckets(0.001, 2.0, 15))
            });
        registry.register(
            "social_api_http_request_duration_seconds",
            "HTTP request duration in seconds",
            http_request_duration.clone(),
        );

        let cache_operations_total = Family::default();
        registry.register(
            "social_api_cache_operations_total",
            "Total cache operations",
            cache_operations_total.clone(),
        );

        let external_calls_total = Family::default();
        registry.register(
            "social_api_external_calls_total",
            "Total external API calls",
            external_calls_total.clone(),
        );

        let external_call_duration =
            Family::<ExternalDurationLabels, Histogram>::new_with_constructor(|| {
                Histogram::new(exponential_buckets(0.001, 2.0, 15))
            });
        registry.register(
            "social_api_external_call_duration_seconds",
            "External API call duration in seconds",
            external_call_duration.clone(),
        );

        let circuit_breaker_state = Family::default();
        registry.register(
            "social_api_circuit_breaker_state",
            "Circuit breaker state (0=closed, 1=open, 2=half-open)",
            circuit_breaker_state.clone(),
        );

        let db_pool_connections = Family::default();
        registry.register(
            "social_api_db_pool_connections",
            "Database pool connections by state",
            db_pool_connections.clone(),
        );

        let sse_connections_active = Gauge::default();
        registry.register(
            "social_api_sse_connections_active",
            "Active SSE connections",
            sse_connections_active.clone(),
        );

        let likes_total = Family::default();
        registry.register(
            "social_api_likes_total",
            "Total like operations",
            likes_total.clone(),
        );

        Arc::new(Self {
            registry,
            http_requests_total,
            http_request_duration,
            cache_operations_total,
            external_calls_total,
            external_call_duration,
            circuit_breaker_state,
            db_pool_connections,
            sse_connections_active,
            likes_total,
        })
    }

    pub fn render(&self) -> String {
        let mut buffer = String::new();
        encode(&mut buffer, &self.registry).unwrap();
        buffer
    }
}
