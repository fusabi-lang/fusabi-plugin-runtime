//! Integration tests for fusabi-plugin-runtime.

use fusabi_plugin_runtime::{
    LifecycleState,
    LoaderConfig, PluginLoader,
    ApiVersion, Dependency, Manifest, ManifestBuilder,
    Plugin, PluginHandle, PluginInfo,
    PluginRegistry, RegistryConfig,
    PluginRuntime, RuntimeConfig,
    Error,
};

// Helper to create test plugins
fn create_test_manifest(name: &str) -> Manifest {
    ManifestBuilder::new(name, "1.0.0")
        .source("test.fsx")
        .export("main")
        .build_unchecked()
}

fn create_test_plugin(name: &str) -> PluginHandle {
    let manifest = create_test_manifest(name);
    PluginHandle::new(Plugin::new(manifest))
}

#[test]
fn test_manifest_creation() {
    let manifest = ManifestBuilder::new("test-plugin", "1.0.0")
        .description("Test plugin")
        .author("Test Author")
        .license("MIT")
        .api_version(ApiVersion::new(0, 21, 0))
        .capability("fs:read")
        .capability("time:read")
        .source("main.fsx")
        .export("init")
        .export("run")
        .tag("test")
        .metadata("key", "value")
        .build()
        .unwrap();

    assert_eq!(manifest.name, "test-plugin");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.capabilities.len(), 2);
    assert!(manifest.requires_capability("fs:read"));
    assert!(manifest.exports.contains(&"init".to_string()));
}

#[test]
fn test_manifest_validation() {
    // Valid manifest
    let manifest = ManifestBuilder::new("valid", "1.0.0")
        .source("test.fsx")
        .build();
    assert!(manifest.is_ok());

    // Missing entry point
    let manifest = ManifestBuilder::new("invalid", "1.0.0").build();
    assert!(manifest.is_err());

    // Invalid capability
    let mut manifest = ManifestBuilder::new("invalid", "1.0.0")
        .source("test.fsx")
        .build_unchecked();
    manifest.capabilities.push("invalid:cap".into());
    assert!(manifest.validate().is_err());
}

#[test]
fn test_api_version_compatibility() {
    let host = ApiVersion::new(0, 21, 5);
    let compatible_same_minor = ApiVersion::new(0, 21, 0);
    let compatible_older_minor = ApiVersion::new(0, 20, 0); // backward compatible
    let incompatible_newer_minor = ApiVersion::new(0, 22, 0); // requires newer host
    let incompatible_major = ApiVersion::new(1, 0, 0);

    // Same minor version - compatible
    assert!(host.is_compatible_with(&compatible_same_minor));
    // Older minor version - backward compatible (plugin for 0.20 works on 0.21 host)
    assert!(host.is_compatible_with(&compatible_older_minor));
    // Newer minor version - incompatible (plugin for 0.22 won't work on 0.21 host)
    assert!(!host.is_compatible_with(&incompatible_newer_minor));
    // Different major version - incompatible
    assert!(!host.is_compatible_with(&incompatible_major));
}

#[test]
fn test_plugin_lifecycle() {
    let manifest = create_test_manifest("lifecycle-test");
    let plugin = Plugin::new(manifest);

    assert_eq!(plugin.state(), LifecycleState::Created);

    // Initialize
    plugin
        .initialize(fusabi_host::EngineConfig::default())
        .unwrap();
    assert_eq!(plugin.state(), LifecycleState::Initialized);

    // Start
    plugin.start().unwrap();
    assert_eq!(plugin.state(), LifecycleState::Running);

    // Stop
    plugin.stop().unwrap();
    assert_eq!(plugin.state(), LifecycleState::Stopped);

    // Unload
    plugin.unload().unwrap();
    assert_eq!(plugin.state(), LifecycleState::Unloaded);
}

#[test]
fn test_plugin_invalid_transitions() {
    let manifest = create_test_manifest("transition-test");
    let plugin = Plugin::new(manifest);

    // Can't start without initialization
    assert!(plugin.start().is_err());

    // Can't stop without starting
    assert!(plugin.stop().is_err());
}

#[test]
fn test_registry_operations() {
    let registry = PluginRegistry::new(RegistryConfig::default());

    // Register plugins
    registry.register(create_test_plugin("plugin-1")).unwrap();
    registry.register(create_test_plugin("plugin-2")).unwrap();

    assert_eq!(registry.len(), 2);
    assert!(registry.contains("plugin-1"));
    assert!(registry.contains("plugin-2"));

    // Get plugin
    let plugin = registry.get("plugin-1").unwrap();
    assert_eq!(plugin.name(), "plugin-1");

    // Unregister
    registry.unregister("plugin-1").unwrap();
    assert!(!registry.contains("plugin-1"));
    assert_eq!(registry.len(), 1);
}

#[test]
fn test_registry_duplicate_prevention() {
    let registry = PluginRegistry::new(RegistryConfig::default());

    registry.register(create_test_plugin("duplicate")).unwrap();
    let result = registry.register(create_test_plugin("duplicate"));

    assert!(matches!(result, Err(Error::PluginAlreadyLoaded(_))));
}

#[test]
fn test_registry_overwrite() {
    let config = RegistryConfig::new().with_allow_overwrite(true);
    let registry = PluginRegistry::new(config);

    let plugin1 = create_test_plugin("overwrite-test");
    let id1 = plugin1.id();

    let plugin2 = create_test_plugin("overwrite-test");
    let id2 = plugin2.id();

    registry.register(plugin1).unwrap();
    registry.register(plugin2).unwrap();

    let plugin = registry.get("overwrite-test").unwrap();
    assert_eq!(plugin.id(), id2);
    assert_ne!(plugin.id(), id1);
}

#[test]
fn test_registry_max_plugins() {
    let config = RegistryConfig::new().with_max_plugins(2);
    let registry = PluginRegistry::new(config);

    registry.register(create_test_plugin("plugin-1")).unwrap();
    registry.register(create_test_plugin("plugin-2")).unwrap();

    let result = registry.register(create_test_plugin("plugin-3"));
    assert!(matches!(result, Err(Error::Registry(_))));
}

#[test]
fn test_registry_stats() {
    let registry = PluginRegistry::new(RegistryConfig::default());

    registry.register(create_test_plugin("plugin-1")).unwrap();
    registry.register(create_test_plugin("plugin-2")).unwrap();

    let stats = registry.stats();
    assert_eq!(stats.total, 2);
}

#[test]
fn test_loader_config() {
    let config = LoaderConfig::new()
        .with_host_api_version(ApiVersion::new(0, 21, 0))
        .with_auto_start(false)
        .with_strict_validation(true);

    assert!(!config.auto_start);
    assert!(config.strict_validation);
}

#[test]
fn test_runtime_creation() {
    let runtime = PluginRuntime::new(RuntimeConfig::default()).unwrap();

    assert_eq!(runtime.plugin_count(), 0);
    assert!(runtime.running().is_empty());
}

#[test]
fn test_runtime_config() {
    let config = RuntimeConfig::new()
        .with_auto_discover(true)
        .with_plugin_dir("/plugins");

    assert!(config.auto_discover);
    assert_eq!(config.plugin_dirs.len(), 1);
}

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    #[test]
    fn test_manifest_toml_roundtrip() {
        let manifest = ManifestBuilder::new("toml-test", "1.0.0")
            .description("Test manifest")
            .api_version(ApiVersion::new(0, 21, 0))
            .capability("fs:read")
            .source("main.fsx")
            .export("main")
            .build_unchecked();

        let toml = manifest.to_toml().unwrap();
        let parsed = Manifest::from_toml(&toml).unwrap();

        assert_eq!(parsed.name, manifest.name);
        assert_eq!(parsed.version, manifest.version);
        assert_eq!(parsed.capabilities, manifest.capabilities);
    }

    #[test]
    fn test_manifest_json_roundtrip() {
        let manifest = ManifestBuilder::new("json-test", "1.0.0")
            .source("main.fsx")
            .export("main")
            .build_unchecked();

        let json = manifest.to_json().unwrap();
        let parsed = Manifest::from_json(&json).unwrap();

        assert_eq!(parsed.name, manifest.name);
    }
}

#[cfg(feature = "watch")]
mod watch_tests {
    use super::*;
    use fusabi_plugin_runtime::{PluginWatcher, WatchConfig, WatchEvent};
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn test_watch_config() {
        let config = WatchConfig::new()
            .with_debounce(Duration::from_secs(1))
            .with_recursive(false)
            .with_auto_reload(true);

        assert_eq!(config.debounce, Duration::from_secs(1));
        assert!(!config.recursive);
        assert!(config.auto_reload);
    }

    #[test]
    fn test_watcher_creation() {
        let watcher = PluginWatcher::default_config().unwrap();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_watch_paths() {
        let mut watcher = PluginWatcher::default_config().unwrap();

        watcher.watch("/tmp/test1").unwrap();
        watcher.watch("/tmp/test2").unwrap();

        let paths = watcher.watched_paths();
        assert_eq!(paths.len(), 2);

        watcher.unwatch("/tmp/test1").unwrap();
        let paths = watcher.watched_paths();
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn test_watch_event_extension_match() {
        let event = WatchEvent::Modified {
            path: PathBuf::from("test.fsx"),
        };

        assert!(event.matches_extension(&["fsx".to_string()]));
        assert!(!event.matches_extension(&["rs".to_string()]));
    }
}
