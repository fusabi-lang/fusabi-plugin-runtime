//! Hot-reload example for Fusabi plugin runtime.
//!
//! This example demonstrates how to watch plugin files for changes
//! and automatically reload them when modifications are detected.
//!
//! Run with: cargo run --example hot_reload --features "serde,watch"

use fusabi_plugin_runtime::{
    PluginRegistry, RegistryConfig, PluginWatcher, WatchConfig, WatchEvent,
};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting hot-reload example");

    // Create plugin registry
    let registry_config = RegistryConfig::new()
        .with_max_plugins(100)
        .with_allow_overwrite(true);

    let _registry = PluginRegistry::new(registry_config);

    // Create watcher with configuration
    let watch_config = WatchConfig::new()
        .with_debounce(Duration::from_millis(500))
        .with_recursive(true)
        .with_auto_reload(true);

    let mut watcher = PluginWatcher::new(watch_config)?;

    // Set up change handler
    watcher.on_change(|event| {
        match &event {
            WatchEvent::Created { path } => {
                info!("Plugin file created: {:?}", path);
            }
            WatchEvent::Modified { path } => {
                info!("Plugin file modified: {:?}", path);
                // In a real application, you would reload the plugin here
            }
            WatchEvent::Removed { path } => {
                info!("Plugin file removed: {:?}", path);
            }
            WatchEvent::Renamed { from, to } => {
                info!("Plugin file renamed: {:?} -> {:?}", from, to);
            }
        }
    });

    // Watch the plugins directory
    let plugins_dir = PathBuf::from("./plugins");
    if plugins_dir.exists() {
        info!("Watching plugins directory: {:?}", plugins_dir);
        watcher.watch(&plugins_dir)?;
    } else {
        info!("Plugins directory not found, create ./plugins/ and add .fsx files");
        info!("Example: mkdir -p ./plugins && echo '// Test plugin' > ./plugins/test.fsx");
    }

    // Start the watcher
    watcher.start()?;

    // Keep running
    info!("Watching for file changes. Press Ctrl+C to exit.");

    loop {
        std::thread::sleep(Duration::from_millis(100));
    }
}
