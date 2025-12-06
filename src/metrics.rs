//! Prometheus metrics integration for plugin runtime.

use prometheus::{Counter, Histogram, Registry};

/// Configuration for plugin metrics collection.
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Metric prefix for all plugin metrics.
    pub prefix: String,
    /// Whether to collect detailed timing histograms.
    pub detailed_timing: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            prefix: "fusabi_plugin".to_string(),
            detailed_timing: true,
        }
    }
}

impl MetricsConfig {
    /// Create a new metrics configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the metric prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Enable or disable detailed timing.
    pub fn with_detailed_timing(mut self, enabled: bool) -> Self {
        self.detailed_timing = enabled;
        self
    }
}

/// Plugin metrics collector.
pub struct PluginMetrics {
    config: MetricsConfig,
    registry: Registry,
    plugins_loaded: Counter,
    plugins_unloaded: Counter,
    plugin_errors: Counter,
    load_duration: Histogram,
    call_duration: Histogram,
}

impl PluginMetrics {
    /// Create a new metrics collector with the given configuration.
    pub fn new(config: MetricsConfig) -> Self {
        let registry = Registry::new();

        let plugins_loaded = Counter::new(
            format!("{}_loaded_total", config.prefix),
            "Total number of plugins loaded",
        )
        .unwrap();

        let plugins_unloaded = Counter::new(
            format!("{}_unloaded_total", config.prefix),
            "Total number of plugins unloaded",
        )
        .unwrap();

        let plugin_errors = Counter::new(
            format!("{}_errors_total", config.prefix),
            "Total number of plugin errors",
        )
        .unwrap();

        let load_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                format!("{}_load_duration_seconds", config.prefix),
                "Plugin load duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
        )
        .unwrap();

        let call_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                format!("{}_call_duration_seconds", config.prefix),
                "Plugin call duration in seconds",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5]),
        )
        .unwrap();

        registry.register(Box::new(plugins_loaded.clone())).ok();
        registry.register(Box::new(plugins_unloaded.clone())).ok();
        registry.register(Box::new(plugin_errors.clone())).ok();
        registry.register(Box::new(load_duration.clone())).ok();
        registry.register(Box::new(call_duration.clone())).ok();

        Self {
            config,
            registry,
            plugins_loaded,
            plugins_unloaded,
            plugin_errors,
            load_duration,
            call_duration,
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &MetricsConfig {
        &self.config
    }

    /// Get the Prometheus registry.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Record a plugin load event.
    pub fn record_load(&self, duration_secs: f64) {
        self.plugins_loaded.inc();
        self.load_duration.observe(duration_secs);
    }

    /// Record a plugin unload event.
    pub fn record_unload(&self) {
        self.plugins_unloaded.inc();
    }

    /// Record a plugin error.
    pub fn record_error(&self) {
        self.plugin_errors.inc();
    }

    /// Record a plugin function call.
    pub fn record_call(&self, duration_secs: f64) {
        self.call_duration.observe(duration_secs);
    }

    /// Get the total number of plugins loaded.
    pub fn plugins_loaded_total(&self) -> u64 {
        self.plugins_loaded.get() as u64
    }

    /// Get the total number of plugins unloaded.
    pub fn plugins_unloaded_total(&self) -> u64 {
        self.plugins_unloaded.get() as u64
    }

    /// Get the total number of plugin errors.
    pub fn plugin_errors_total(&self) -> u64 {
        self.plugin_errors.get() as u64
    }
}

impl std::fmt::Debug for PluginMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginMetrics")
            .field("config", &self.config)
            .field("plugins_loaded", &self.plugins_loaded_total())
            .field("plugins_unloaded", &self.plugins_unloaded_total())
            .field("plugin_errors", &self.plugin_errors_total())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_config_builder() {
        let config = MetricsConfig::new()
            .with_prefix("test")
            .with_detailed_timing(false);

        assert_eq!(config.prefix, "test");
        assert!(!config.detailed_timing);
    }

    #[test]
    fn test_metrics_recording() {
        let metrics = PluginMetrics::new(MetricsConfig::default());

        metrics.record_load(0.1);
        metrics.record_load(0.2);
        metrics.record_unload();
        metrics.record_error();
        metrics.record_call(0.01);

        assert_eq!(metrics.plugins_loaded_total(), 2);
        assert_eq!(metrics.plugins_unloaded_total(), 1);
        assert_eq!(metrics.plugin_errors_total(), 1);
    }
}
