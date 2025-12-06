# Migration Guide (vNEXT)

Guide for migrating to new versions of fusabi-plugin-runtime.

## Table of Contents

- [Overview](#overview)
- [Version History](#version-history)
- [Breaking Changes](#breaking-changes)
- [Deprecated Features](#deprecated-features)
- [New Features](#new-features)
- [Migration Examples](#migration-examples)
- [Troubleshooting](#troubleshooting)

## Overview

This guide helps you migrate your code when upgrading fusabi-plugin-runtime versions.

### Migration Strategy

1. Review breaking changes for your version
2. Update deprecated API usage
3. Test with new features
4. Update manifests if needed
5. Run full test suite

## Version History

### v0.1.0 (Current)

Initial release with core functionality:
- Plugin loading from source and bytecode
- Manifest validation
- Capability enforcement
- Hot reload (with `watch` feature)
- Plugin registry
- Lifecycle management

### vNEXT (Unreleased)

Planned enhancements:
- Multi-process sandboxing
- Host profiles (terminal, observability, orchestration)
- Hot reload debounce and backoff
- Metrics hooks and tracing spans
- Enhanced manifest schema (UI vs daemon plugins)
- Improved error handling

## Breaking Changes

### From v0.1.0 to vNEXT

#### 1. LoaderConfig API Changes

**Before (v0.1.0)**:
```rust
let config = LoaderConfig::default()
    .with_capabilities(capabilities);

let loader = PluginLoader::new(config)?;
```

**After (vNEXT)**:
```rust
use fusabi_host::EngineConfig;

let config = LoaderConfig::new()
    .with_engine_config(
        EngineConfig::default()
            .with_capabilities(capabilities)
    );

let loader = PluginLoader::new(config)?;
```

**Rationale**: Engine configuration moved to separate type for better organization.

#### 2. Manifest Field Changes

**Before (v0.1.0)**:
```toml
# API version as string
api_version = "0.18.0"
```

**After (vNEXT)**:
```toml
# API version as structured object
api-version = { major = 0, minor = 18, patch = 0 }
```

**Migration Script**:
```rust
// Convert old manifests
fn migrate_manifest(old: &str) -> String {
    old.replace(
        r#"api_version = "(\d+)\.(\d+)\.(\d+)""#,
        r#"api-version = { major = $1, minor = $2, patch = $3 }"#
    )
}
```

#### 3. Plugin Call API

**Before (v0.1.0)**:
```rust
let result = plugin.call("function_name", vec![arg1, arg2])?;
```

**After (vNEXT)**:
```rust
let result = plugin.call("function_name", &[arg1, arg2])?;
```

**Rationale**: Use slice instead of Vec for efficiency.

#### 4. Error Types

**Before (v0.1.0)**:
```rust
match error {
    Error::Io(e) => { /* ... */ }
    Error::Parse(e) => { /* ... */ }
}
```

**After (vNEXT)**:
```rust
match error {
    Error::Io(e) => { /* ... */ }
    Error::ManifestParse(msg) => { /* ... */ }
    Error::InvalidManifest(msg) => { /* ... */ }
    Error::IncompatibleApiVersion { required, host } => { /* ... */ }
}
```

**Rationale**: More specific error variants for better error handling.

## Deprecated Features

### v0.1.0 Deprecations

#### 1. `PluginLoader::load` (deprecated in v0.1.0, removed in vNEXT)

**Deprecated**:
```rust
let plugin = loader.load("plugin.fsx")?;
```

**Use instead**:
```rust
// Explicitly choose source or bytecode
let plugin = loader.load_source("plugin.fsx")?;
// or
let plugin = loader.load_bytecode_file("plugin.fzb")?;
```

#### 2. `Manifest::from_str` (deprecated in v0.1.0, removed in vNEXT)

**Deprecated**:
```rust
let manifest = Manifest::from_str(content)?;
```

**Use instead**:
```rust
// Explicitly choose format
let manifest = Manifest::from_toml(content)?;
// or
let manifest = Manifest::from_json(content)?;
```

#### 3. Global Registry (deprecated in v0.1.0, removed in vNEXT)

**Deprecated**:
```rust
fusabi_plugin_runtime::register_plugin(plugin)?;
let plugin = fusabi_plugin_runtime::get_plugin("name")?;
```

**Use instead**:
```rust
// Create explicit registry
let registry = PluginRegistry::new(RegistryConfig::default());
registry.register(plugin)?;
let plugin = registry.get("name")?;
```

## New Features

### vNEXT Additions

#### 1. Host Profiles

New capability presets for common scenarios:

```rust
use fusabi_plugin_runtime::profiles::{
    terminal_capabilities,
    observability_capabilities,
    orchestration_capabilities,
};

// Use profile
let config = LoaderConfig::new()
    .with_engine_config(
        EngineConfig::default()
            .with_capabilities(terminal_capabilities())
    );
```

See [Host Profiles](host-profiles.md) for details.

#### 2. Hot Reload Debounce

Configure debouncing for hot reload:

```rust
use std::time::Duration;

let config = WatchConfig::new()
    .with_debounce(Duration::from_millis(500))
    .with_backoff(BackoffConfig::exponential()
        .with_max_retries(5)
    );
```

See [Hot Reload](hot-reload.md) for details.

#### 3. Metrics Hooks

Track plugin performance:

```rust
use fusabi_plugin_runtime::metrics::PluginMetrics;

let metrics = PluginMetrics::new();

runtime.on_call(|plugin, function, duration| {
    metrics.record_call(plugin, function, duration);
});

// Export metrics
let summary = metrics.summary();
println!("Total calls: {}", summary.total_calls);
println!("Average duration: {:?}", summary.avg_duration);
```

#### 4. Manifest Metadata

Add custom metadata to manifests:

```toml
[metadata]
category = "ui"
interactive = "true"
homepage = "https://example.com"
```

```rust
let manifest = Manifest::from_file("plugin.toml")?;
let category = manifest.metadata.get("category");
```

## Migration Examples

### Example 1: Update Loader Code

**Before (v0.1.0)**:
```rust
use fusabi_plugin_runtime::{PluginLoader, LoaderConfig};
use fusabi_host::Capabilities;

fn create_loader() -> Result<PluginLoader> {
    let mut caps = Capabilities::safe_defaults();
    caps.grant(Capability::FileWrite);

    let config = LoaderConfig::default()
        .with_capabilities(caps);

    PluginLoader::new(config)
}
```

**After (vNEXT)**:
```rust
use fusabi_plugin_runtime::{PluginLoader, LoaderConfig};
use fusabi_host::{Capabilities, Capability, EngineConfig};

fn create_loader() -> Result<PluginLoader> {
    let mut caps = Capabilities::safe_defaults();
    caps.grant(Capability::FileWrite);

    let config = LoaderConfig::new()
        .with_engine_config(
            EngineConfig::default()
                .with_capabilities(caps)
        );

    PluginLoader::new(config)
}
```

### Example 2: Update Manifest Files

**Before (v0.1.0)**:
```toml
name = "my-plugin"
version = "1.0.0"
api_version = "0.18.0"
capabilities = ["fs:read"]
source = "main.fsx"
```

**After (vNEXT)**:
```toml
name = "my-plugin"
version = "1.0.0"
api-version = { major = 0, minor = 18, patch = 0 }
capabilities = ["fs:read"]
source = "main.fsx"

# Optional: Add metadata
[metadata]
category = "utility"
```

### Example 3: Update Plugin Calls

**Before (v0.1.0)**:
```rust
let args = vec![
    Value::String("arg1".into()),
    Value::Number(42.0),
];
let result = plugin.call("process", args)?;
```

**After (vNEXT)**:
```rust
let result = plugin.call("process", &[
    Value::String("arg1".into()),
    Value::Number(42.0),
])?;
```

### Example 4: Update Error Handling

**Before (v0.1.0)**:
```rust
match loader.load_from_manifest("plugin.toml") {
    Ok(plugin) => { /* ... */ }
    Err(Error::Parse(msg)) => {
        eprintln!("Parse error: {}", msg);
    }
    Err(e) => { /* ... */ }
}
```

**After (vNEXT)**:
```rust
match loader.load_from_manifest("plugin.toml") {
    Ok(plugin) => { /* ... */ }
    Err(Error::ManifestParse(msg)) => {
        eprintln!("Manifest parse error: {}", msg);
    }
    Err(Error::InvalidManifest(msg)) => {
        eprintln!("Invalid manifest: {}", msg);
    }
    Err(Error::IncompatibleApiVersion { required, host }) => {
        eprintln!("API version mismatch: need {}, have {}", required, host);
    }
    Err(e) => { /* ... */ }
}
```

### Example 5: Adopt Host Profiles

**Before (v0.1.0)**:
```rust
// Manual capability configuration
let mut caps = Capabilities::none();
caps.grant(Capability::FileRead);
caps.grant(Capability::FileMetadata);
caps.grant(Capability::NetworkRequest);
caps.grant(Capability::TimeRead);

let config = LoaderConfig::default()
    .with_capabilities(caps);
```

**After (vNEXT)**:
```rust
// Use profile
use fusabi_plugin_runtime::profiles::observability_capabilities;

let config = LoaderConfig::new()
    .with_engine_config(
        EngineConfig::default()
            .with_capabilities(observability_capabilities())
    );
```

## Troubleshooting

### Common Migration Issues

#### Issue 1: Compilation Errors After Update

**Symptom**:
```
error[E0061]: this function takes 1 argument but 2 were supplied
```

**Solution**: Check API changes in breaking changes section. Update method calls to match new signatures.

#### Issue 2: Manifest Loading Fails

**Symptom**:
```
Error: ManifestParse("missing field `api-version`")
```

**Solution**: Update manifest format. Change `api_version = "X.Y.Z"` to structured format:
```toml
api-version = { major = X, minor = Y, patch = Z }
```

#### Issue 3: Capability Denied Errors

**Symptom**:
```
Error: CapabilityDenied("fs:write")
```

**Solution**: Capabilities may need explicit grant. Check that your config grants all required capabilities:
```rust
let mut caps = Capabilities::safe_defaults();
caps.grant(Capability::FileWrite);  // Explicitly grant
```

#### Issue 4: Plugin Not Hot Reloading

**Symptom**: File changes don't trigger reload

**Solution**: Ensure watcher is configured and started:
```rust
let mut watcher = PluginWatcher::new(config)?;
watcher.watch("plugins/")?;
watcher.start()?;  // Don't forget to start!
```

#### Issue 5: Type Mismatch Errors

**Symptom**:
```
error[E0308]: mismatched types
expected `&[Value]`, found `Vec<Value>`
```

**Solution**: Change `Vec` to slice reference:
```rust
// Before
plugin.call("func", vec![arg])?;

// After
plugin.call("func", &[arg])?;
```

### Getting Help

If you encounter issues not covered here:

1. Check the [changelog](../../CHANGELOG.md)
2. Review the [documentation](runtime-guide.md)
3. Search [existing issues](https://github.com/fusabi-lang/fusabi-plugin-runtime/issues)
4. Open a [new issue](https://github.com/fusabi-lang/fusabi-plugin-runtime/issues/new)

## See Also

- [Runtime Guide](runtime-guide.md)
- [Manifest Schema](manifest-schema.md)
- [Capabilities Guide](capabilities.md)
- [Host Profiles](host-profiles.md)
