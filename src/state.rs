use std::{sync::Arc, time::Duration};

use crate::{
    constants::RUNTIMEBACKENDS_INTERVAL_CHECK, forwarders::runtime_backends::RuntimeBackends,
    statistics::metrics::ProxyMetrics,
};

#[derive(Clone)]
pub struct AppState {
    pub metrics: ProxyMetrics,
    // List of backends disabled at runtime because they have become inaccessible
    pub runtime_disabled_backend: Arc<RuntimeBackends>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        let instance = Arc::new(Self {
            metrics: ProxyMetrics::new(),
            runtime_disabled_backend: RuntimeBackends::new(Duration::from_secs(
                RUNTIMEBACKENDS_INTERVAL_CHECK,
            )),
        });
        instance
    }
}
