//! Plugin runtime for managing the plugin lifecycle.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::error::{Error, Result};
use crate::lifecycle::{LifecycleHooks, LifecycleState};
use crate::loader::{LoaderConfig, PluginLoader};
use crate::plugin::PluginHandle;
use crate::registry::{PluginRegistry, RegistryConfig, RegistryStats};

/// Configuration for the plugin runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Loader configuration.
    pub loader: LoaderConfig,
    /// Registry configuration.
    pub registry: RegistryConfig,
    /// Plugin directories to scan.
    pub plugin_dirs: Vec<PathBuf>,
    /// Whether to auto-discover plugins.
    pub auto_discover: bool,
    /// File patterns to match for plugins.
    pub plugin_patterns: Vec<String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            loader: LoaderConfig::default(),
            registry: RegistryConfig::default(),
            plugin_dirs: Vec::new(),
            auto_discover: false,
            plugin_patterns: vec![
                "*.toml".to_string(),
                "plugin.toml".to_string(),
                "fusabi.toml".to_string(),
            ],
        }
    }
}

impl RuntimeConfig {
    /// Create a new runtime configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the loader configuration.
    pub fn with_loader(mut self, loader: LoaderConfig) -> Self {
        self.loader = loader;
        self
    }

    /// Set the registry configuration.
    pub fn with_registry(mut self, registry: RegistryConfig) -> Self {
        self.registry = registry;
        self
    }

    /// Add a plugin directory.
    pub fn with_plugin_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.plugin_dirs.push(dir.into());
        self
    }

    /// Enable auto-discovery.
    pub fn with_auto_discover(mut self, auto: bool) -> Self {
        self.auto_discover = auto;
        self
    }

    /// Set plugin patterns.
    pub fn with_plugin_patterns(mut self, patterns: Vec<String>) -> Self {
        self.plugin_patterns = patterns;
        self
    }
}

/// Plugin runtime for managing plugins.
pub struct PluginRuntime {
    config: RuntimeConfig,
    loader: PluginLoader,
    registry: PluginRegistry,
    hooks: Arc<RwLock<LifecycleHooks>>,
}

impl PluginRuntime {
    /// Create a new plugin runtime.
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        let loader = PluginLoader::new(config.loader.clone())?;
        let registry = PluginRegistry::new(config.registry.clone());

        Ok(Self {
            config,
            loader,
            registry,
            hooks: Arc::new(RwLock::new(LifecycleHooks::new())),
        })
    }

    /// Create with default configuration.
    pub fn default_config() -> Result<Self> {
        Self::new(RuntimeConfig::default())
    }

    /// Get the runtime configuration.
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Get the plugin loader.
    pub fn loader(&self) -> &PluginLoader {
        &self.loader
    }

    /// Get the plugin registry.
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    /// Add a lifecycle event handler.
    pub fn on_event<F>(&self, handler: F)
    where
        F: Fn(&crate::lifecycle::LifecycleEvent) + Send + Sync + 'static,
    {
        self.hooks.write().on_event(handler);
    }

    /// Load a plugin from a manifest file.
    #[cfg(feature = "serde")]
    pub fn load_manifest(&self, path: impl Into<PathBuf>) -> Result<PluginHandle> {
        let plugin = self.loader.load_from_manifest(path.into())?;
        self.registry.register(plugin.clone())?;
        Ok(plugin)
    }

    /// Load a plugin from source.
    pub fn load_source(&self, path: impl Into<PathBuf>) -> Result<PluginHandle> {
        let plugin = self.loader.load_source(path.into())?;
        self.registry.register(plugin.clone())?;
        Ok(plugin)
    }

    /// Load a plugin from bytecode.
    pub fn load_bytecode(&self, path: impl Into<PathBuf>) -> Result<PluginHandle> {
        let plugin = self.loader.load_bytecode_file(path.into())?;
        self.registry.register(plugin.clone())?;
        Ok(plugin)
    }

    /// Unload a plugin by name.
    pub fn unload(&self, name: &str) -> Result<()> {
        self.registry.unregister(name)?;
        Ok(())
    }

    /// Get a plugin by name.
    pub fn get(&self, name: &str) -> Option<PluginHandle> {
        self.registry.get(name)
    }

    /// Check if a plugin is loaded.
    pub fn has_plugin(&self, name: &str) -> bool {
        self.registry.contains(name)
    }

    /// Get all loaded plugins.
    pub fn plugins(&self) -> Vec<PluginHandle> {
        self.registry.all()
    }

    /// Get running plugins.
    pub fn running(&self) -> Vec<PluginHandle> {
        self.registry.running()
    }

    /// Get plugin count.
    pub fn plugin_count(&self) -> usize {
        self.registry.len()
    }

    /// Get registry statistics.
    pub fn stats(&self) -> RegistryStats {
        self.registry.stats()
    }

    /// Start a plugin.
    pub fn start(&self, name: &str) -> Result<()> {
        let plugin = self
            .registry
            .get(name)
            .ok_or_else(|| Error::plugin_not_found(name))?;

        plugin.inner().start()?;
        self.hooks.read().emit_started(name);

        Ok(())
    }

    /// Stop a plugin.
    pub fn stop(&self, name: &str) -> Result<()> {
        let plugin = self
            .registry
            .get(name)
            .ok_or_else(|| Error::plugin_not_found(name))?;

        plugin.inner().stop()?;
        self.hooks.read().emit_stopped(name);

        Ok(())
    }

    /// Reload a plugin.
    pub fn reload(&self, name: &str) -> Result<()> {
        self.registry.reload(name)
    }

    /// Start all plugins.
    pub fn start_all(&self) -> Vec<Result<()>> {
        self.registry.start_all()
    }

    /// Stop all plugins.
    pub fn stop_all(&self) -> Vec<Result<()>> {
        self.registry.stop_all()
    }

    /// Reload all plugins.
    pub fn reload_all(&self) -> Vec<Result<()>> {
        self.registry.reload_all()
    }

    /// Discover and load plugins from configured directories.
    #[cfg(feature = "serde")]
    pub fn discover(&self) -> Result<Vec<PluginHandle>> {
        let mut loaded = Vec::new();

        for dir in &self.config.plugin_dirs {
            if !dir.exists() {
                tracing::warn!("Plugin directory does not exist: {}", dir.display());
                continue;
            }

            for pattern in &self.config.plugin_patterns {
                let glob_pattern = dir.join(pattern);
                let glob_str = glob_pattern.to_string_lossy();

                if let Ok(entries) = glob::glob(&glob_str) {
                    for entry in entries.flatten() {
                        match self.load_manifest(&entry) {
                            Ok(plugin) => {
                                tracing::info!(
                                    "Loaded plugin {} from {}",
                                    plugin.name(),
                                    entry.display()
                                );
                                loaded.push(plugin);
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to load plugin from {}: {}",
                                    entry.display(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Call a function on a plugin.
    pub fn call(
        &self,
        plugin_name: &str,
        function: &str,
        args: &[fusabi_host::Value],
    ) -> Result<fusabi_host::Value> {
        let plugin = self
            .registry
            .get(plugin_name)
            .ok_or_else(|| Error::plugin_not_found(plugin_name))?;

        plugin.call(function, args)
    }

    /// Broadcast a function call to all running plugins.
    pub fn broadcast(
        &self,
        function: &str,
        args: &[fusabi_host::Value],
    ) -> Vec<(String, Result<fusabi_host::Value>)> {
        self.registry
            .running()
            .into_iter()
            .filter(|p| p.has_export(function))
            .map(|p| {
                let name = p.name();
                let result = p.call(function, args);
                (name, result)
            })
            .collect()
    }

    /// Clean up unloaded plugins.
    pub fn cleanup(&self) -> usize {
        self.registry.cleanup()
    }

    /// Shutdown the runtime.
    pub fn shutdown(&self) {
        // Stop all running plugins
        self.stop_all();

        // Unload all
        self.registry.unload_all();
    }
}

impl std::fmt::Debug for PluginRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRuntime")
            .field("config", &self.config)
            .field("plugin_count", &self.registry.len())
            .finish()
    }
}

impl Drop for PluginRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = PluginRuntime::default_config().unwrap();
        assert_eq!(runtime.plugin_count(), 0);
    }

    #[test]
    fn test_runtime_config_builder() {
        let config = RuntimeConfig::new()
            .with_plugin_dir("/plugins")
            .with_auto_discover(true);

        assert_eq!(config.plugin_dirs.len(), 1);
        assert!(config.auto_discover);
    }

    #[test]
    fn test_runtime_stats() {
        let runtime = PluginRuntime::default_config().unwrap();
        let stats = runtime.stats();

        assert_eq!(stats.total, 0);
        assert_eq!(stats.running, 0);
    }
}

// glob is an optional dependency for discovery
#[cfg(feature = "serde")]
mod glob {
    pub fn glob(pattern: &str) -> std::io::Result<impl Iterator<Item = std::io::Result<std::path::PathBuf>>> {
        // Simplified glob implementation for testing
        // In production, would use the actual glob crate
        Ok(std::iter::empty())
    }
}
