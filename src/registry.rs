//! Plugin registry for managing loaded plugins.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;

use crate::error::{Error, Result};
use crate::lifecycle::{LifecycleHooks, LifecycleState};
use crate::plugin::{Plugin, PluginHandle, PluginInfo};

/// Configuration for the plugin registry.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Maximum number of plugins allowed.
    pub max_plugins: usize,
    /// Whether to allow plugin overwrites.
    pub allow_overwrite: bool,
    /// Whether to automatically unload stopped plugins.
    pub auto_unload_stopped: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_plugins: 100,
            allow_overwrite: false,
            auto_unload_stopped: false,
        }
    }
}

impl RegistryConfig {
    /// Create a new registry configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of plugins.
    pub fn with_max_plugins(mut self, max: usize) -> Self {
        self.max_plugins = max;
        self
    }

    /// Allow plugin overwrites.
    pub fn with_allow_overwrite(mut self, allow: bool) -> Self {
        self.allow_overwrite = allow;
        self
    }

    /// Enable auto-unload for stopped plugins.
    pub fn with_auto_unload_stopped(mut self, auto: bool) -> Self {
        self.auto_unload_stopped = auto;
        self
    }
}

/// Registry statistics.
#[derive(Debug, Clone, Default)]
pub struct RegistryStats {
    /// Total plugins registered.
    pub total: usize,
    /// Plugins currently running.
    pub running: usize,
    /// Plugins stopped.
    pub stopped: usize,
    /// Plugins in error state.
    pub error: usize,
    /// Plugins unloaded.
    pub unloaded: usize,
}

/// Plugin registry for managing loaded plugins.
pub struct PluginRegistry {
    config: RegistryConfig,
    plugins: DashMap<String, PluginHandle>,
    hooks: Arc<LifecycleHooks>,
}

impl PluginRegistry {
    /// Create a new plugin registry.
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            config,
            plugins: DashMap::new(),
            hooks: Arc::new(LifecycleHooks::new()),
        }
    }

    /// Create with default configuration.
    pub fn default_config() -> Self {
        Self::new(RegistryConfig::default())
    }

    /// Get the registry configuration.
    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }

    /// Register a plugin.
    pub fn register(&self, plugin: PluginHandle) -> Result<()> {
        let name = plugin.name();

        // Check capacity
        if self.plugins.len() >= self.config.max_plugins {
            return Err(Error::Registry(format!(
                "registry full: max {} plugins",
                self.config.max_plugins
            )));
        }

        // Check for existing
        if self.plugins.contains_key(&name) {
            if !self.config.allow_overwrite {
                return Err(Error::PluginAlreadyLoaded(name));
            }

            // Unload existing
            if let Some((_, existing)) = self.plugins.remove(&name) {
                let _ = existing.inner().unload();
            }
        }

        self.plugins.insert(name.clone(), plugin);
        self.hooks.emit_created(&name);

        Ok(())
    }

    /// Unregister a plugin by name.
    pub fn unregister(&self, name: &str) -> Result<PluginHandle> {
        let (_, plugin) = self
            .plugins
            .remove(name)
            .ok_or_else(|| Error::plugin_not_found(name))?;

        // Unload the plugin
        let _ = plugin.inner().unload();
        self.hooks.emit_unloaded(name);

        Ok(plugin)
    }

    /// Get a plugin by name.
    pub fn get(&self, name: &str) -> Option<PluginHandle> {
        self.plugins.get(name).map(|r| r.clone())
    }

    /// Check if a plugin exists.
    pub fn contains(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    /// Get all plugin names.
    pub fn names(&self) -> Vec<String> {
        self.plugins.iter().map(|r| r.key().clone()).collect()
    }

    /// Get all plugins.
    pub fn all(&self) -> Vec<PluginHandle> {
        self.plugins.iter().map(|r| r.value().clone()).collect()
    }

    /// Get plugins by state.
    pub fn by_state(&self, state: LifecycleState) -> Vec<PluginHandle> {
        self.plugins
            .iter()
            .filter(|r| r.state() == state)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get running plugins.
    pub fn running(&self) -> Vec<PluginHandle> {
        self.by_state(LifecycleState::Running)
    }

    /// Get plugin count.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Get registry statistics.
    pub fn stats(&self) -> RegistryStats {
        let mut stats = RegistryStats::default();
        stats.total = self.plugins.len();

        for entry in self.plugins.iter() {
            match entry.state() {
                LifecycleState::Running => stats.running += 1,
                LifecycleState::Stopped => stats.stopped += 1,
                LifecycleState::Error => stats.error += 1,
                LifecycleState::Unloaded => stats.unloaded += 1,
                _ => {}
            }
        }

        stats
    }

    /// Get all plugin info.
    pub fn info(&self) -> Vec<PluginInfo> {
        self.plugins.iter().map(|r| r.info()).collect()
    }

    /// Start all stopped plugins.
    pub fn start_all(&self) -> Vec<Result<()>> {
        self.plugins
            .iter()
            .filter(|r| r.state() == LifecycleState::Initialized)
            .map(|r| {
                let plugin = r.value();
                plugin.inner().start()
            })
            .collect()
    }

    /// Stop all running plugins.
    pub fn stop_all(&self) -> Vec<Result<()>> {
        self.plugins
            .iter()
            .filter(|r| r.state() == LifecycleState::Running)
            .map(|r| {
                let plugin = r.value();
                plugin.inner().stop()
            })
            .collect()
    }

    /// Unload all plugins.
    pub fn unload_all(&self) {
        for entry in self.plugins.iter() {
            let _ = entry.value().inner().unload();
        }
        self.plugins.clear();
    }

    /// Reload a plugin by name.
    pub fn reload(&self, name: &str) -> Result<()> {
        let plugin = self
            .get(name)
            .ok_or_else(|| Error::plugin_not_found(name))?;

        plugin.inner().reload()?;

        let info = plugin.info();
        self.hooks.emit_reloaded(name, info.reload_count);

        Ok(())
    }

    /// Reload all plugins.
    pub fn reload_all(&self) -> Vec<Result<()>> {
        self.plugins
            .iter()
            .map(|r| {
                let name = r.key().clone();
                self.reload(&name)
            })
            .collect()
    }

    /// Find plugins by tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<PluginHandle> {
        self.plugins
            .iter()
            .filter(|r| {
                r.value()
                    .inner()
                    .manifest()
                    .tags
                    .contains(&tag.to_string())
            })
            .map(|r| r.value().clone())
            .collect()
    }

    /// Find plugins by capability.
    pub fn find_by_capability(&self, cap: &str) -> Vec<PluginHandle> {
        self.plugins
            .iter()
            .filter(|r| r.value().inner().requires_capability(cap))
            .map(|r| r.value().clone())
            .collect()
    }

    /// Clean up unloaded and error plugins.
    pub fn cleanup(&self) -> usize {
        let to_remove: Vec<String> = self
            .plugins
            .iter()
            .filter(|r| {
                let state = r.state();
                state == LifecycleState::Unloaded
                    || (self.config.auto_unload_stopped && state == LifecycleState::Stopped)
            })
            .map(|r| r.key().clone())
            .collect();

        let count = to_remove.len();
        for name in to_remove {
            self.plugins.remove(&name);
        }

        count
    }
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("config", &self.config)
            .field("plugin_count", &self.plugins.len())
            .finish()
    }
}

impl Drop for PluginRegistry {
    fn drop(&mut self) {
        // Unload all plugins on drop
        for entry in self.plugins.iter() {
            let _ = entry.value().inner().unload();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ManifestBuilder;

    fn create_test_plugin(name: &str) -> PluginHandle {
        let manifest = ManifestBuilder::new(name, "1.0.0")
            .source("test.fsx")
            .build_unchecked();
        PluginHandle::new(Plugin::new(manifest))
    }

    #[test]
    fn test_registry_creation() {
        let registry = PluginRegistry::default_config();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_register_plugin() {
        let registry = PluginRegistry::default_config();
        let plugin = create_test_plugin("test-plugin");

        registry.register(plugin).unwrap();

        assert!(registry.contains("test-plugin"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_register_duplicate() {
        let registry = PluginRegistry::default_config();

        let plugin1 = create_test_plugin("test-plugin");
        let plugin2 = create_test_plugin("test-plugin");

        registry.register(plugin1).unwrap();
        let result = registry.register(plugin2);

        assert!(matches!(result, Err(Error::PluginAlreadyLoaded(_))));
    }

    #[test]
    fn test_register_duplicate_with_overwrite() {
        let config = RegistryConfig::new().with_allow_overwrite(true);
        let registry = PluginRegistry::new(config);

        let plugin1 = create_test_plugin("test-plugin");
        let id1 = plugin1.id();

        let plugin2 = create_test_plugin("test-plugin");
        let id2 = plugin2.id();

        registry.register(plugin1).unwrap();
        registry.register(plugin2).unwrap();

        let plugin = registry.get("test-plugin").unwrap();
        assert_eq!(plugin.id(), id2);
        assert_ne!(plugin.id(), id1);
    }

    #[test]
    fn test_unregister_plugin() {
        let registry = PluginRegistry::default_config();
        let plugin = create_test_plugin("test-plugin");

        registry.register(plugin).unwrap();
        assert!(registry.contains("test-plugin"));

        registry.unregister("test-plugin").unwrap();
        assert!(!registry.contains("test-plugin"));
    }

    #[test]
    fn test_unregister_nonexistent() {
        let registry = PluginRegistry::default_config();
        let result = registry.unregister("nonexistent");
        assert!(matches!(result, Err(Error::PluginNotFound(_))));
    }

    #[test]
    fn test_get_all_plugins() {
        let registry = PluginRegistry::default_config();

        registry.register(create_test_plugin("plugin-1")).unwrap();
        registry.register(create_test_plugin("plugin-2")).unwrap();
        registry.register(create_test_plugin("plugin-3")).unwrap();

        let all = registry.all();
        assert_eq!(all.len(), 3);

        let names = registry.names();
        assert!(names.contains(&"plugin-1".to_string()));
        assert!(names.contains(&"plugin-2".to_string()));
        assert!(names.contains(&"plugin-3".to_string()));
    }

    #[test]
    fn test_registry_stats() {
        let registry = PluginRegistry::default_config();

        registry.register(create_test_plugin("plugin-1")).unwrap();
        registry.register(create_test_plugin("plugin-2")).unwrap();

        let stats = registry.stats();
        assert_eq!(stats.total, 2);
    }

    #[test]
    fn test_max_plugins() {
        let config = RegistryConfig::new().with_max_plugins(2);
        let registry = PluginRegistry::new(config);

        registry.register(create_test_plugin("plugin-1")).unwrap();
        registry.register(create_test_plugin("plugin-2")).unwrap();

        let result = registry.register(create_test_plugin("plugin-3"));
        assert!(matches!(result, Err(Error::Registry(_))));
    }
}
