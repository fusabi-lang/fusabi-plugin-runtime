//! # fusabi-plugin-runtime
//!
//! Plugin loader, hot-reload, and runtime for Fusabi plugins (fsx & fzb)
//! with manifest validation and capability enforcement.
//!
//! This crate provides:
//! - **Plugin Loading** - Load plugins from source (.fsx) or bytecode (.fzb)
//! - **Manifest Validation** - Validate plugin manifests and enforce requirements
//! - **Capability Enforcement** - Ensure plugins only use declared capabilities
//! - **Hot Reload** - Automatically reload plugins when files change
//! - **Lifecycle Management** - Initialize, run, and cleanup plugins
//! - **Metrics Hooks** - Track plugin performance and usage
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use fusabi_plugin_runtime::{PluginLoader, LoaderConfig, PluginRegistry};
//!
//! // Create a loader
//! let loader = PluginLoader::new(LoaderConfig::default())?;
//!
//! // Load a plugin from manifest
//! let plugin = loader.load_from_manifest("plugin.toml")?;
//!
//! // Call plugin functions
//! let result = plugin.call("main", &[])?;
//! ```
//!
//! ## Feature Flags
//!
//! - `serde` (default): Enable manifest parsing and serialization
//! - `watch`: Enable filesystem watching for hot reload
//! - `metrics-prometheus`: Prometheus metrics integration

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod error;
mod lifecycle;
mod loader;
mod manifest;
mod plugin;
mod registry;
mod runtime;

#[cfg(feature = "watch")]
mod watcher;

#[cfg(feature = "metrics-prometheus")]
mod metrics;

pub use error::{Error, Result};
pub use lifecycle::{PluginLifecycle, LifecycleState, LifecycleHooks};
pub use loader::{PluginLoader, LoaderConfig};
pub use manifest::{Manifest, ManifestBuilder, ApiVersion, Dependency};
pub use plugin::{Plugin, PluginInfo, PluginHandle};
pub use registry::{PluginRegistry, RegistryConfig};
pub use runtime::{PluginRuntime, RuntimeConfig};

#[cfg(feature = "watch")]
pub use watcher::{PluginWatcher, WatchConfig, WatchEvent};

#[cfg(feature = "metrics-prometheus")]
pub use metrics::{PluginMetrics, MetricsConfig};

// Re-export key types from fusabi-host for convenience
pub use fusabi_host::{
    Capabilities, Capability, Limits, Value, Error as HostError,
};

/// Crate version for compatibility checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
