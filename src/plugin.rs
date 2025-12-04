//! Plugin representation and execution.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;

use fusabi_host::{Engine, EngineConfig, Value};

use crate::error::{Error, Result};
use crate::lifecycle::LifecycleState;
use crate::manifest::Manifest;

static NEXT_PLUGIN_ID: AtomicU64 = AtomicU64::new(1);

/// Information about a loaded plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Unique plugin ID.
    pub id: u64,
    /// Plugin name from manifest.
    pub name: String,
    /// Plugin version from manifest.
    pub version: String,
    /// Path to the manifest file.
    pub manifest_path: Option<PathBuf>,
    /// Path to the source/bytecode file.
    pub entry_path: Option<PathBuf>,
    /// When the plugin was loaded.
    pub loaded_at: Instant,
    /// When the plugin was last reloaded.
    pub last_reload: Option<Instant>,
    /// Total reload count.
    pub reload_count: u64,
    /// Total invocation count.
    pub invocation_count: u64,
    /// Current lifecycle state.
    pub state: LifecycleState,
}

impl PluginInfo {
    /// Create new plugin info.
    fn new(id: u64, manifest: &Manifest) -> Self {
        Self {
            id,
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            manifest_path: None,
            entry_path: None,
            loaded_at: Instant::now(),
            last_reload: None,
            reload_count: 0,
            invocation_count: 0,
            state: LifecycleState::Created,
        }
    }
}

/// Internal plugin state.
struct PluginInner {
    manifest: Manifest,
    info: PluginInfo,
    engine: Option<Engine>,
    bytecode: Option<Vec<u8>>,
}

/// A loaded Fusabi plugin.
pub struct Plugin {
    inner: RwLock<PluginInner>,
}

impl Plugin {
    /// Create a new plugin from a manifest.
    pub fn new(manifest: Manifest) -> Self {
        let id = NEXT_PLUGIN_ID.fetch_add(1, Ordering::Relaxed);
        let info = PluginInfo::new(id, &manifest);

        Self {
            inner: RwLock::new(PluginInner {
                manifest,
                info,
                engine: None,
                bytecode: None,
            }),
        }
    }

    /// Get the plugin ID.
    pub fn id(&self) -> u64 {
        self.inner.read().info.id
    }

    /// Get the plugin name.
    pub fn name(&self) -> String {
        self.inner.read().manifest.name.clone()
    }

    /// Get the plugin version.
    pub fn version(&self) -> String {
        self.inner.read().manifest.version.clone()
    }

    /// Get the plugin manifest.
    pub fn manifest(&self) -> Manifest {
        self.inner.read().manifest.clone()
    }

    /// Get plugin information.
    pub fn info(&self) -> PluginInfo {
        self.inner.read().info.clone()
    }

    /// Get the current lifecycle state.
    pub fn state(&self) -> LifecycleState {
        self.inner.read().info.state
    }

    /// Set the lifecycle state.
    pub fn set_state(&self, state: LifecycleState) {
        self.inner.write().info.state = state;
    }

    /// Initialize the plugin with an engine.
    pub fn initialize(&self, engine_config: EngineConfig) -> Result<()> {
        let mut inner = self.inner.write();

        // Check state
        if inner.info.state != LifecycleState::Created
            && inner.info.state != LifecycleState::Stopped
        {
            return Err(Error::invalid_state(
                "Created or Stopped",
                format!("{:?}", inner.info.state),
            ));
        }

        // Verify capabilities
        let caps = &engine_config.capabilities;
        for required_cap in &inner.manifest.capabilities {
            let cap = fusabi_host::Capability::from_name(required_cap)
                .ok_or_else(|| Error::invalid_manifest(format!("unknown capability: {}", required_cap)))?;

            if !caps.has(cap) {
                return Err(Error::MissingCapability(required_cap.clone()));
            }
        }

        // Create engine
        let engine = Engine::new(engine_config)
            .map_err(|e| Error::init_failed(e.to_string()))?;

        inner.engine = Some(engine);
        inner.info.state = LifecycleState::Initialized;

        Ok(())
    }

    /// Start the plugin (call init function if exists).
    pub fn start(&self) -> Result<()> {
        let mut inner = self.inner.write();

        if inner.info.state != LifecycleState::Initialized {
            return Err(Error::invalid_state(
                "Initialized",
                format!("{:?}", inner.info.state),
            ));
        }

        // Call init function if declared
        if inner.manifest.exports.contains(&"init".to_string()) {
            if let Some(ref engine) = inner.engine {
                engine
                    .execute("init()")
                    .map_err(|e| Error::init_failed(e.to_string()))?;
            }
        }

        inner.info.state = LifecycleState::Running;
        Ok(())
    }

    /// Stop the plugin (call cleanup function if exists).
    pub fn stop(&self) -> Result<()> {
        let mut inner = self.inner.write();

        if inner.info.state != LifecycleState::Running {
            return Err(Error::invalid_state(
                "Running",
                format!("{:?}", inner.info.state),
            ));
        }

        // Call cleanup function if declared
        if inner.manifest.exports.contains(&"cleanup".to_string()) {
            if let Some(ref engine) = inner.engine {
                let _ = engine.execute("cleanup()");
            }
        }

        inner.info.state = LifecycleState::Stopped;
        Ok(())
    }

    /// Unload the plugin.
    pub fn unload(&self) -> Result<()> {
        let mut inner = self.inner.write();

        // Try to stop if running
        if inner.info.state == LifecycleState::Running {
            if inner.manifest.exports.contains(&"cleanup".to_string()) {
                if let Some(ref engine) = inner.engine {
                    let _ = engine.execute("cleanup()");
                }
            }
        }

        inner.engine = None;
        inner.bytecode = None;
        inner.info.state = LifecycleState::Unloaded;

        Ok(())
    }

    /// Call a function exported by the plugin.
    pub fn call(&self, function: &str, args: &[Value]) -> Result<Value> {
        let mut inner = self.inner.write();

        // Check state
        if inner.info.state != LifecycleState::Running {
            return Err(Error::invalid_state(
                "Running",
                format!("{:?}", inner.info.state),
            ));
        }

        // Check function is exported
        if !inner.manifest.exports.contains(&function.to_string())
            && function != "main"
        {
            return Err(Error::FunctionNotFound(function.to_string()));
        }

        // Build call expression
        let call_expr = if args.is_empty() {
            format!("{}()", function)
        } else {
            // Format args - simplified for simulation
            let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
            format!("{}({})", function, args_str.join(", "))
        };

        // Execute
        let engine = inner
            .engine
            .as_ref()
            .ok_or_else(|| Error::invalid_state("engine initialized", "no engine"))?;

        inner.info.invocation_count += 1;

        engine
            .execute(&call_expr)
            .map_err(|e| Error::execution_failed(e.to_string()))
    }

    /// Reload the plugin from source.
    pub fn reload(&self) -> Result<()> {
        let mut inner = self.inner.write();

        // Must be in a reloadable state
        if inner.info.state == LifecycleState::Unloaded {
            return Err(Error::PluginUnloaded);
        }

        let was_running = inner.info.state == LifecycleState::Running;

        // Stop if running
        if was_running {
            if inner.manifest.exports.contains(&"cleanup".to_string()) {
                if let Some(ref engine) = inner.engine {
                    let _ = engine.execute("cleanup()");
                }
            }
        }

        // Reset state
        inner.info.state = LifecycleState::Initialized;
        inner.info.last_reload = Some(Instant::now());
        inner.info.reload_count += 1;

        // Restart if was running
        if was_running {
            inner.info.state = LifecycleState::Running;
            if inner.manifest.exports.contains(&"init".to_string()) {
                if let Some(ref engine) = inner.engine {
                    engine
                        .execute("init()")
                        .map_err(|e| Error::ReloadFailed(e.to_string()))?;
                }
            }
        }

        Ok(())
    }

    /// Check if the plugin exports a function.
    pub fn has_export(&self, name: &str) -> bool {
        self.inner.read().manifest.exports.contains(&name.to_string())
    }

    /// Get all exported function names.
    pub fn exports(&self) -> Vec<String> {
        self.inner.read().manifest.exports.clone()
    }

    /// Check if the plugin requires a capability.
    pub fn requires_capability(&self, cap: &str) -> bool {
        self.inner.read().manifest.requires_capability(cap)
    }

    /// Set the compiled bytecode.
    pub fn set_bytecode(&self, bytecode: Vec<u8>) {
        self.inner.write().bytecode = Some(bytecode);
    }

    /// Get the compiled bytecode if available.
    pub fn bytecode(&self) -> Option<Vec<u8>> {
        self.inner.read().bytecode.clone()
    }
}

impl std::fmt::Debug for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read();
        f.debug_struct("Plugin")
            .field("id", &inner.info.id)
            .field("name", &inner.manifest.name)
            .field("version", &inner.manifest.version)
            .field("state", &inner.info.state)
            .finish()
    }
}

/// Handle to a loaded plugin for safe concurrent access.
#[derive(Clone)]
pub struct PluginHandle {
    plugin: Arc<Plugin>,
}

impl PluginHandle {
    /// Create a new plugin handle.
    pub fn new(plugin: Plugin) -> Self {
        Self {
            plugin: Arc::new(plugin),
        }
    }

    /// Get the plugin ID.
    pub fn id(&self) -> u64 {
        self.plugin.id()
    }

    /// Get the plugin name.
    pub fn name(&self) -> String {
        self.plugin.name()
    }

    /// Get the plugin state.
    pub fn state(&self) -> LifecycleState {
        self.plugin.state()
    }

    /// Call a function on the plugin.
    pub fn call(&self, function: &str, args: &[Value]) -> Result<Value> {
        self.plugin.call(function, args)
    }

    /// Get plugin info.
    pub fn info(&self) -> PluginInfo {
        self.plugin.info()
    }

    /// Check if the plugin exports a function.
    pub fn has_export(&self, name: &str) -> bool {
        self.plugin.has_export(name)
    }

    /// Get the underlying plugin.
    pub fn inner(&self) -> &Plugin {
        &self.plugin
    }
}

impl std::fmt::Debug for PluginHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginHandle")
            .field("id", &self.id())
            .field("name", &self.name())
            .field("state", &self.state())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ManifestBuilder;

    fn create_test_manifest() -> Manifest {
        ManifestBuilder::new("test-plugin", "1.0.0")
            .source("test.fsx")
            .export("main")
            .export("init")
            .build_unchecked()
    }

    #[test]
    fn test_plugin_creation() {
        let manifest = create_test_manifest();
        let plugin = Plugin::new(manifest);

        assert!(plugin.id() > 0);
        assert_eq!(plugin.name(), "test-plugin");
        assert_eq!(plugin.version(), "1.0.0");
        assert_eq!(plugin.state(), LifecycleState::Created);
    }

    #[test]
    fn test_plugin_lifecycle() {
        let manifest = create_test_manifest();
        let plugin = Plugin::new(manifest);

        // Initialize
        plugin
            .initialize(EngineConfig::default())
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
    fn test_plugin_invalid_state_transitions() {
        let manifest = create_test_manifest();
        let plugin = Plugin::new(manifest);

        // Can't start before initialize
        assert!(plugin.start().is_err());

        // Can't stop before start
        assert!(plugin.stop().is_err());

        // Initialize first
        plugin.initialize(EngineConfig::default()).unwrap();

        // Can't stop before start
        assert!(plugin.stop().is_err());
    }

    #[test]
    fn test_plugin_capabilities() {
        let manifest = ManifestBuilder::new("test", "1.0.0")
            .source("test.fsx")
            .capability("fs:read")
            .build_unchecked();

        let plugin = Plugin::new(manifest);

        // Missing capability should fail
        let config = EngineConfig::default()
            .with_capabilities(fusabi_host::Capabilities::none());

        assert!(plugin.initialize(config).is_err());

        // With capability should succeed
        let config = EngineConfig::default().with_capabilities(
            fusabi_host::Capabilities::none().with(fusabi_host::Capability::FsRead),
        );

        assert!(plugin.initialize(config).is_ok());
    }

    #[test]
    fn test_plugin_handle() {
        let manifest = create_test_manifest();
        let plugin = Plugin::new(manifest);
        let handle = PluginHandle::new(plugin);

        assert!(handle.id() > 0);
        assert_eq!(handle.name(), "test-plugin");
        assert!(handle.has_export("main"));

        // Clone and verify
        let handle2 = handle.clone();
        assert_eq!(handle.id(), handle2.id());
    }
}
