# Runtime Guide (vNEXT)

This guide covers the fusabi-plugin-runtime system for loading, managing, and executing Fusabi plugins.

## Table of Contents

- [Quick Start](#quick-start)
- [Plugin Loading](#plugin-loading)
- [Plugin Registry](#plugin-registry)
- [Lifecycle Management](#lifecycle-management)
- [Configuration](#configuration)
- [Error Handling](#error-handling)
- [Examples](#examples)

## Quick Start

The plugin runtime provides a high-level API for working with Fusabi plugins:

```rust
use fusabi_plugin_runtime::{PluginRuntime, RuntimeConfig};

fn main() -> fusabi_plugin_runtime::Result<()> {
    // Create runtime with default configuration
    let runtime = PluginRuntime::new(RuntimeConfig::default())?;

    // Load a plugin from its manifest
    let plugin = runtime.load_manifest("plugins/example/plugin.toml")?;

    // Call a plugin function
    let result = runtime.call("example", "process", &[Value::String("data".into())])?;

    println!("Result: {:?}", result);

    Ok(())
}
```

## Plugin Loading

The runtime supports multiple ways to load plugins:

### From Manifest File

The recommended approach uses a plugin manifest (TOML):

```rust
use fusabi_plugin_runtime::{PluginLoader, LoaderConfig};

let loader = PluginLoader::new(LoaderConfig::default())?;
let plugin = loader.load_from_manifest("plugin.toml")?;
```

Example manifest:
```toml
name = "my-plugin"
version = "1.0.0"
description = "Example plugin"
api-version = { major = 0, minor = 18, patch = 0 }
capabilities = ["fs:read", "net:request"]
source = "main.fsx"
exports = ["init", "process", "cleanup"]
```

### From Source File

Load directly from Fusabi source code (.fsx):

```rust
let plugin = loader.load_source("plugin.fsx")?;
```

### From Bytecode

Load pre-compiled bytecode (.fzb):

```rust
let plugin = loader.load_bytecode_file("plugin.fzb")?;
```

### From Memory

Load bytecode from memory:

```rust
let bytecode: Vec<u8> = /* ... */;
let plugin = loader.load_bytecode(&bytecode)?;
```

## Plugin Registry

The registry manages multiple plugins with concurrent access:

```rust
use fusabi_plugin_runtime::{PluginRegistry, RegistryConfig};

let registry = PluginRegistry::new(RegistryConfig::default());

// Register a plugin
registry.register(plugin)?;

// Get plugin by name
let plugin = registry.get("my-plugin")?;

// Check if plugin exists
if registry.contains("my-plugin") {
    println!("Plugin is registered");
}

// List all running plugins
let running = registry.running();
for name in running {
    println!("Running: {}", name);
}

// Reload a plugin
registry.reload("my-plugin")?;

// Unregister a plugin
registry.unregister("my-plugin")?;
```

### Thread Safety

The registry uses `DashMap` for concurrent access:

```rust
use std::thread;

let registry = Arc::new(PluginRegistry::new(RegistryConfig::default()));

// Use from multiple threads
let handles: Vec<_> = (0..4)
    .map(|i| {
        let registry = Arc::clone(&registry);
        thread::spawn(move || {
            registry.get("plugin").map(|p| {
                // Use plugin
            })
        })
    })
    .collect();

for handle in handles {
    handle.join().unwrap();
}
```

## Lifecycle Management

Plugins have a managed lifecycle with hooks:

```rust
use fusabi_plugin_runtime::{PluginLifecycle, LifecycleState, LifecycleHooks};

// Create lifecycle manager
let mut lifecycle = PluginLifecycle::new(plugin);

// Add hooks
let hooks = LifecycleHooks::new()
    .on_init(|plugin_name| {
        println!("Initializing {}", plugin_name);
    })
    .on_start(|plugin_name| {
        println!("Starting {}", plugin_name);
    })
    .on_stop(|plugin_name| {
        println!("Stopping {}", plugin_name);
    })
    .on_cleanup(|plugin_name| {
        println!("Cleaning up {}", plugin_name);
    });

lifecycle.set_hooks(hooks);

// Manage lifecycle
lifecycle.init()?;
lifecycle.start()?;

// Check state
assert_eq!(lifecycle.state(), LifecycleState::Running);

// Stop and cleanup
lifecycle.stop()?;
lifecycle.cleanup()?;
```

### Lifecycle States

- **Created**: Plugin loaded but not initialized
- **Initialized**: Init function called successfully
- **Running**: Plugin is active and processing
- **Stopped**: Plugin stopped but cleanup not called
- **Failed**: Plugin encountered an error

## Configuration

### Loader Configuration

```rust
use fusabi_plugin_runtime::{LoaderConfig, ApiVersion};
use fusabi_host::{Capabilities, Limits, EngineConfig};

let config = LoaderConfig::new()
    .with_host_api_version(ApiVersion::new(0, 18, 0))
    .with_engine_config(
        EngineConfig::default()
            .with_capabilities(Capabilities::safe_defaults())
            .with_limits(Limits {
                max_memory: 10 * 1024 * 1024, // 10MB
                max_call_depth: 100,
                max_instructions: 1_000_000,
            })
    )
    .with_manifest_dir("plugins/");

let loader = PluginLoader::new(config)?;
```

### Runtime Configuration

```rust
use fusabi_plugin_runtime::RuntimeConfig;

let config = RuntimeConfig::new()
    .with_loader_config(loader_config)
    .with_registry_config(registry_config)
    .with_max_concurrent_calls(100)
    .with_call_timeout(std::time::Duration::from_secs(30));

let runtime = PluginRuntime::new(config)?;
```

### Registry Configuration

```rust
use fusabi_plugin_runtime::RegistryConfig;

let config = RegistryConfig::new()
    .with_max_plugins(50)
    .with_allow_duplicates(false);

let registry = PluginRegistry::new(config);
```

## Error Handling

The runtime uses a comprehensive error type:

```rust
use fusabi_plugin_runtime::{Error, Result};

fn load_plugin() -> Result<()> {
    let loader = PluginLoader::new(LoaderConfig::default())?;

    match loader.load_from_manifest("plugin.toml") {
        Ok(plugin) => {
            println!("Loaded: {}", plugin.name());
            Ok(())
        }
        Err(Error::ManifestNotFound(path)) => {
            eprintln!("Manifest not found: {}", path);
            Err(Error::ManifestNotFound(path))
        }
        Err(Error::ManifestParse(msg)) => {
            eprintln!("Invalid manifest: {}", msg);
            Err(Error::ManifestParse(msg))
        }
        Err(Error::IncompatibleApiVersion { required, host }) => {
            eprintln!("API version mismatch: required {}, host {}", required, host);
            Err(Error::IncompatibleApiVersion { required, host })
        }
        Err(e) => Err(e),
    }
}
```

### Error Categories

- **IO Errors**: File not found, permissions, etc.
- **Parse Errors**: Invalid manifest, malformed source
- **Validation Errors**: Missing fields, invalid capabilities
- **Compatibility Errors**: API version mismatch
- **Runtime Errors**: Plugin execution failures
- **State Errors**: Invalid lifecycle transitions

## Examples

### Complete Plugin Loading Pipeline

```rust
use fusabi_plugin_runtime::{
    PluginRuntime, RuntimeConfig, LoaderConfig,
    ApiVersion, RegistryConfig
};
use fusabi_host::{Capabilities, Value};

fn main() -> fusabi_plugin_runtime::Result<()> {
    // Configure loader
    let loader_config = LoaderConfig::new()
        .with_host_api_version(ApiVersion::new(0, 18, 0))
        .with_manifest_dir("plugins/");

    // Configure registry
    let registry_config = RegistryConfig::new()
        .with_max_plugins(10);

    // Create runtime
    let runtime = PluginRuntime::new(
        RuntimeConfig::new()
            .with_loader_config(loader_config)
            .with_registry_config(registry_config)
    )?;

    // Load plugins from directory
    let plugins = runtime.load_directory("plugins/")?;
    println!("Loaded {} plugins", plugins.len());

    // Initialize all plugins
    for name in plugins {
        runtime.init_plugin(&name)?;
    }

    // Call a plugin function
    let result = runtime.call("data-processor", "process", &[
        Value::String("input.csv".into()),
    ])?;

    println!("Processing result: {:?}", result);

    // Cleanup
    runtime.shutdown()?;

    Ok(())
}
```

### Hot Reload with Watcher

See [hot-reload.md](hot-reload.md) for detailed hot reload documentation.

```rust
use fusabi_plugin_runtime::{PluginWatcher, WatchConfig, WatchEvent};

fn watch_plugins(runtime: PluginRuntime) -> fusabi_plugin_runtime::Result<()> {
    let mut watcher = PluginWatcher::new(WatchConfig::default())?;

    watcher.on_change(move |event| {
        match event {
            WatchEvent::Modified { path } => {
                if let Some(plugin_name) = extract_plugin_name(&path) {
                    println!("Reloading {}", plugin_name);
                    runtime.reload_plugin(&plugin_name).ok();
                }
            }
            _ => {}
        }
    });

    watcher.watch("plugins/")?;
    watcher.start()?;

    Ok(())
}
```

### Capability Enforcement

See [capabilities.md](capabilities.md) for detailed capability documentation.

```rust
use fusabi_host::{Capabilities, Capability};

let mut caps = Capabilities::none();
caps.grant(Capability::FileRead);
caps.grant(Capability::TimeRead);

let config = LoaderConfig::new()
    .with_engine_config(
        EngineConfig::default().with_capabilities(caps)
    );

let loader = PluginLoader::new(config)?;

// Plugin requesting fs:write will fail to load
let result = loader.load_from_manifest("restricted-plugin.toml");
assert!(result.is_err());
```

## Best Practices

1. **Always validate manifests** before loading plugins
2. **Use capability gating** to restrict plugin access
3. **Set appropriate resource limits** to prevent abuse
4. **Handle lifecycle transitions** gracefully
5. **Monitor plugin performance** with metrics hooks
6. **Test hot reload behavior** in development
7. **Version your plugin APIs** to maintain compatibility

## See Also

- [Capabilities Guide](capabilities.md)
- [Host Profiles](host-profiles.md)
- [Manifest Schema](manifest-schema.md)
- [Hot Reload](hot-reload.md)
- [Multi-Process Safety](multi-process.md)
- [Migration Guide](migration.md)
