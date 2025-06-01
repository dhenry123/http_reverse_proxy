use crate::statistics::metrics::ProxyMetrics;

#[derive(Clone)]
pub struct AppState {
    pub metrics: ProxyMetrics,
}
