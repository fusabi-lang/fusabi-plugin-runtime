# fusabi-plugin-runtime

Plugin loader, hot-reload, and runtime for Fusabi plugins (fsx & fzb) with manifest validation and capability enforcement.

## Features

- **Plugin Loading** - Load plugins from source (.fsx) or bytecode (.fzb)
- **Manifest Validation** - Validate plugin manifests and enforce requirements
- **Capability Enforcement** - Ensure plugins only use declared capabilities
- **Hot Reload** - Automatically reload plugins when files change
- **Lifecycle Management** - Initialize, run, and cleanup plugins
- **Plugin Registry** - Manage multiple plugins with concurrent access

## Quick Start

```rust
use fusabi_plugin_runtime::{PluginRuntime, RuntimeConfig};

fn main() -> fusabi_plugin_runtime::Result<()> {
    // Create runtime
    let runtime = PluginRuntime::new(RuntimeConfig::default())?;

    // Load a plugin from manifest
    let plugin = runtime.load_manifest("plugins/my-plugin/plugin.toml")?;

    // Call a function
    let result = runtime.call("my-plugin", "process", &[])?;
    println!("Result: {}", result);

    Ok(())
}
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `serde` (default) | Enable manifest parsing and serialization |
| `watch` | Enable filesystem watching for hot reload |
| `metrics-prometheus` | Prometheus metrics integration |

## Plugin Manifest

Plugins are defined by a TOML manifest:

```toml
name = "my-plugin"
version = "1.0.0"
description = "Example plugin"
api-version = { major = 0, minor = 18, patch = 0 }

# Required capabilities
capabilities = ["fs:read", "net:request"]

# Dependencies
[[dependencies]]
name = "json"
version = "^1.0"

# Entry point
source = "main.fsx"

# Exported functions
exports = ["init", "process", "cleanup"]

# Tags for categorization
tags = ["processing", "example"]
```

## Loading Plugins

```rust
use fusabi_plugin_runtime::{PluginLoader, LoaderConfig};

let loader = PluginLoader::new(LoaderConfig::default())?;

// From manifest
let plugin = loader.load_from_manifest("plugin.toml")?;

// From source directly
let plugin = loader.load_source("plugin.fsx")?;

// From bytecode
let plugin = loader.load_bytecode_file("plugin.fzb")?;
```

## Plugin Registry

```rust
use fusabi_plugin_runtime::{PluginRegistry, RegistryConfig};

let registry = PluginRegistry::new(RegistryConfig::default());

// Register plugins
registry.register(plugin)?;

// Get by name
let plugin = registry.get("my-plugin")?;

// Get all running plugins
let running = registry.running();

// Reload a plugin
registry.reload("my-plugin")?;
```

## Hot Reload

```rust
use fusabi_plugin_runtime::{PluginWatcher, WatchConfig, WatchEvent};

let mut watcher = PluginWatcher::new(WatchConfig::default())?;

// Add change handler
watcher.on_change(|event| {
    match event {
        WatchEvent::Modified { path } => {
            println!("File modified: {}", path.display());
            // Trigger reload
        }
        _ => {}
    }
});

// Watch plugin directories
watcher.watch("plugins/")?;
watcher.start()?;
```

## Lifecycle Hooks

```rust
use fusabi_plugin_runtime::{PluginRuntime, RuntimeConfig};

let runtime = PluginRuntime::new(RuntimeConfig::default())?;

// Add lifecycle event handler
runtime.on_event(|event| {
    println!("Plugin {}: {}", event.plugin_name(), event.event_name());
});
```

## Capability Enforcement

Plugins must declare required capabilities in their manifest. The runtime validates that:

1. All declared capabilities are valid
2. The host grants the required capabilities
3. Plugins don't access undeclared capabilities

```rust
use fusabi_plugin_runtime::{LoaderConfig, Capabilities};

let config = LoaderConfig::new()
    .with_engine_config(
        EngineConfig::default()
            .with_capabilities(Capabilities::safe_defaults())
    );

// Plugin requesting fs:write without it being granted will fail
let loader = PluginLoader::new(config)?;
```

## API Version Compatibility

Plugins declare a required API version. The runtime checks compatibility:

```rust
use fusabi_plugin_runtime::{LoaderConfig, ApiVersion};

let config = LoaderConfig::new()
    .with_host_api_version(ApiVersion::new(0, 18, 0));

// Plugin requiring 0.19.0 will fail to load
```

## Documentation

Comprehensive documentation is available in versioned format:

- **[Runtime Guide](docs/versions/vNEXT/runtime-guide.md)** - Complete guide for using the plugin runtime
- **[Capabilities System](docs/versions/vNEXT/capabilities.md)** - Capability enforcement and security
- **[Host Profiles](docs/versions/vNEXT/host-profiles.md)** - Terminal, observability, and orchestration profiles
- **[Manifest Schema](docs/versions/vNEXT/manifest-schema.md)** - Plugin manifest reference (UI vs daemon)
- **[Hot Reload](docs/versions/vNEXT/hot-reload.md)** - Hot reload with debounce and backoff
- **[Multi-Process Safety](docs/versions/vNEXT/multi-process.md)** - Multi-process sandboxing guide
- **[Migration Guide](docs/versions/vNEXT/migration.md)** - Migrating between versions

### For Contributors

- **[Release Process](docs/RELEASE.md)** - How to cut a release
- **[Documentation Structure](docs/STRUCTURE.md)** - Documentation organization

## Contributing

Contributions are welcome! Please see [CODEOWNERS](.github/CODEOWNERS) for review requirements.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
