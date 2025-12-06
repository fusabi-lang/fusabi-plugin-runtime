# Host Profiles (vNEXT)

Host profiles define standardized capability sets and behaviors for different plugin execution contexts.

## Table of Contents

- [Overview](#overview)
- [Profile Types](#profile-types)
- [Capability Sets](#capability-sets)
- [Plugin Selection](#plugin-selection)
- [Integration Patterns](#integration-patterns)
- [Custom Profiles](#custom-profiles)

## Overview

Host profiles provide:
- **Standardized capability sets** for common use cases
- **Security boundaries** appropriate to each context
- **Plugin compatibility** guidelines
- **Integration patterns** for host applications

### Profile Architecture

```
Host Application
       ↓
Profile Selection (terminal, observability, orchestration)
       ↓
Capability Set Assignment
       ↓
Plugin Loading with Profile Constraints
       ↓
Runtime Enforcement
```

## Profile Types

### Terminal Profile

**Purpose**: Interactive command-line tools and user-facing applications

**Use Cases**:
- CLI plugins
- Interactive data processing
- User-initiated automation
- Development tools

**Characteristics**:
- User oversight and approval
- Interactive feedback
- Moderate security constraints
- Performance-oriented

**Typical Plugins**:
- File format converters
- Code generators
- Data transformers
- Build tool extensions

### Observability Profile

**Purpose**: Monitoring, logging, and telemetry systems

**Use Cases**:
- Metrics collectors
- Log processors
- Trace aggregators
- Health check agents

**Characteristics**:
- Read-mostly operations
- Network reporting
- Strict resource limits
- High reliability requirements

**Typical Plugins**:
- Custom metric exporters
- Log formatters
- Alert processors
- Dashboard data providers

### Orchestration Profile

**Purpose**: Workflow automation and system management

**Use Cases**:
- CI/CD pipelines
- Deployment automation
- System provisioning
- Task scheduling

**Characteristics**:
- Elevated privileges
- Multi-step workflows
- Error recovery
- Audit logging

**Typical Plugins**:
- Deployment scripts
- Infrastructure provisioners
- Backup managers
- Configuration managers

## Capability Sets

### Terminal Profile Capabilities

```rust
use fusabi_host::{Capabilities, Capability};

pub fn terminal_capabilities() -> Capabilities {
    let mut caps = Capabilities::safe_defaults(); // fs:read, fs:metadata, sys:time

    // Add interactive capabilities
    caps.grant(Capability::FileWrite);      // User can see file changes
    caps.grant(Capability::NetworkRequest); // User can monitor network
    caps.grant(Capability::EnvRead);        // Access to user environment
    caps.grant(Capability::DnsLookup);      // DNS resolution

    // Explicitly NOT granted:
    // - fs:delete (too destructive)
    // - sys:exec (security risk)
    // - net:listen (not needed for CLI)

    caps
}
```

**Rationale**: Terminal users can observe plugin behavior and interrupt dangerous operations. File writes are allowed because users can review changes. Network access enables CLI tools that interact with APIs.

### Observability Profile Capabilities

```rust
pub fn observability_capabilities() -> Capabilities {
    let mut caps = Capabilities::none();

    // Read-only filesystem access
    caps.grant(Capability::FileRead);      // Read log files
    caps.grant(Capability::FileMetadata);  // Check file sizes, timestamps

    // Network for metrics reporting
    caps.grant(Capability::NetworkRequest); // Send metrics to backend
    caps.grant(Capability::DnsLookup);      // Resolve metric endpoints

    // Time for timestamps
    caps.grant(Capability::TimeRead);       // Metric timestamps

    // Explicitly NOT granted:
    // - fs:write (observability should not modify system)
    // - fs:delete (observability should not alter system)
    // - sys:exec (no need to run external processes)
    // - sys:env (avoid leaking sensitive environment)

    caps
}
```

**Rationale**: Observability plugins should observe without affecting the system. Read-only access prevents accidental or malicious modifications. Network access is tightly scoped to metrics reporting.

### Orchestration Profile Capabilities

```rust
pub fn orchestration_capabilities() -> Capabilities {
    let mut caps = Capabilities::all();

    // Orchestration needs broad access for automation
    // All capabilities granted, but usage is logged and audited

    caps
}

// Alternative: Orchestration with constraints
pub fn orchestration_capabilities_constrained() -> Capabilities {
    let mut caps = Capabilities::safe_defaults();

    caps.grant(Capability::FileWrite);
    caps.grant(Capability::FileDelete);
    caps.grant(Capability::NetworkRequest);
    caps.grant(Capability::NetworkListen);
    caps.grant(Capability::EnvRead);
    caps.grant(Capability::ProcessExec);

    // With limits
    caps.set_max_file_size(100 * 1024 * 1024); // 100MB max file
    caps.set_max_network_connections(50);
    caps.set_max_processes(10);

    caps
}
```

**Rationale**: Orchestration often requires elevated privileges for automation tasks. However, all operations should be logged for audit trails. Consider using constrained version for production.

## Plugin Selection

### Manifest Tags for Profile Selection

Plugins can declare their intended profile:

```toml
# Terminal plugin example
name = "file-converter"
version = "1.0.0"
capabilities = ["fs:read", "fs:write", "sys:time"]
tags = ["terminal", "converter", "cli"]

[metadata]
profile = "terminal"
interactive = true
```

```toml
# Observability plugin example
name = "log-processor"
version = "1.0.0"
capabilities = ["fs:read", "net:request", "sys:time"]
tags = ["observability", "logging", "metrics"]

[metadata]
profile = "observability"
read-only = true
```

```toml
# Orchestration plugin example
name = "deployer"
version = "1.0.0"
capabilities = ["fs:read", "fs:write", "fs:delete", "net:request", "sys:exec"]
tags = ["orchestration", "deployment", "automation"]

[metadata]
profile = "orchestration"
privileged = true
```

### Profile Validation

```rust
use fusabi_plugin_runtime::{Manifest, Error};

fn validate_profile_compatibility(
    manifest: &Manifest,
    profile: &str
) -> Result<(), Error> {
    let profile_caps = match profile {
        "terminal" => terminal_capabilities(),
        "observability" => observability_capabilities(),
        "orchestration" => orchestration_capabilities(),
        _ => return Err(Error::invalid_manifest("unknown profile")),
    };

    // Check all required capabilities are granted by profile
    for cap_name in &manifest.capabilities {
        let cap = Capability::from_name(cap_name)
            .ok_or_else(|| Error::invalid_manifest("unknown capability"))?;

        if !profile_caps.has(cap) {
            return Err(Error::IncompatibleProfile {
                plugin: manifest.name.clone(),
                profile: profile.to_string(),
                capability: cap_name.clone(),
            });
        }
    }

    Ok(())
}
```

### Profile-Based Plugin Loading

```rust
use fusabi_plugin_runtime::{PluginLoader, LoaderConfig};
use fusabi_host::EngineConfig;

fn load_for_profile(
    manifest_path: &str,
    profile: &str
) -> fusabi_plugin_runtime::Result<Plugin> {
    let manifest = Manifest::from_file(manifest_path)?;

    // Validate plugin is compatible with profile
    validate_profile_compatibility(&manifest, profile)?;

    // Get capabilities for profile
    let caps = match profile {
        "terminal" => terminal_capabilities(),
        "observability" => observability_capabilities(),
        "orchestration" => orchestration_capabilities(),
        _ => return Err(Error::invalid_manifest("unknown profile")),
    };

    let config = LoaderConfig::new()
        .with_engine_config(
            EngineConfig::default()
                .with_capabilities(caps)
        );

    let loader = PluginLoader::new(config)?;
    loader.load_from_manifest(manifest_path)
}
```

## Integration Patterns

### Terminal Integration

```rust
use fusabi_plugin_runtime::{PluginRuntime, RuntimeConfig, LoaderConfig};
use fusabi_host::EngineConfig;
use std::io::{self, Write};

fn terminal_integration() -> Result<()> {
    let config = RuntimeConfig::new()
        .with_loader_config(
            LoaderConfig::new()
                .with_engine_config(
                    EngineConfig::default()
                        .with_capabilities(terminal_capabilities())
                )
        );

    let runtime = PluginRuntime::new(config)?;

    // Load plugins from user's plugin directory
    let home = std::env::var("HOME")?;
    let plugin_dir = format!("{}/.fusabi/plugins", home);

    let plugins = runtime.load_directory(&plugin_dir)?;
    println!("Loaded {} plugins", plugins.len());

    // Interactive plugin selection
    println!("\nAvailable plugins:");
    for (i, name) in plugins.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }

    print!("\nSelect plugin: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let selection: usize = input.trim().parse()?;

    if let Some(plugin_name) = plugins.get(selection - 1) {
        println!("Running {}...", plugin_name);
        let result = runtime.call(plugin_name, "main", &[])?;
        println!("Result: {:?}", result);
    }

    Ok(())
}
```

### Observability Integration

```rust
use fusabi_plugin_runtime::{PluginRuntime, RuntimeConfig};
use std::time::Duration;
use tokio::time;

async fn observability_integration() -> Result<()> {
    let config = RuntimeConfig::new()
        .with_loader_config(
            LoaderConfig::new()
                .with_engine_config(
                    EngineConfig::default()
                        .with_capabilities(observability_capabilities())
                )
        )
        .with_call_timeout(Duration::from_secs(5)); // Short timeout for metrics

    let runtime = PluginRuntime::new(config)?;

    // Load observability plugins
    runtime.load_manifest("/etc/fusabi/plugins/metrics-exporter/plugin.toml")?;
    runtime.load_manifest("/etc/fusabi/plugins/log-processor/plugin.toml")?;

    // Run metrics collection loop
    let mut interval = time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        // Collect metrics from all plugins
        for plugin_name in runtime.list_plugins() {
            if let Ok(metrics) = runtime.call(&plugin_name, "collect_metrics", &[]) {
                println!("Metrics from {}: {:?}", plugin_name, metrics);

                // Send to metrics backend
                // send_to_prometheus(metrics).await?;
            }
        }
    }
}
```

### Orchestration Integration

```rust
use fusabi_plugin_runtime::{PluginRuntime, RuntimeConfig, LifecycleHooks};
use tracing::{info, warn, error};

fn orchestration_integration() -> Result<()> {
    let config = RuntimeConfig::new()
        .with_loader_config(
            LoaderConfig::new()
                .with_engine_config(
                    EngineConfig::default()
                        .with_capabilities(orchestration_capabilities())
                )
        );

    let runtime = PluginRuntime::new(config)?;

    // Add audit logging
    runtime.add_hooks(LifecycleHooks::new()
        .on_call(|plugin, func, args| {
            info!(
                plugin = plugin,
                function = func,
                args = ?args,
                "Plugin function called"
            );
        })
        .on_error(|plugin, error| {
            error!(
                plugin = plugin,
                error = ?error,
                "Plugin error"
            );
        })
    );

    // Load orchestration plugins
    runtime.load_directory("/var/lib/fusabi/orchestration/")?;

    // Execute deployment workflow
    let steps = vec![
        ("validate", "validate_config"),
        ("prepare", "prepare_environment"),
        ("deploy", "deploy_application"),
        ("verify", "verify_deployment"),
    ];

    for (plugin, function) in steps {
        info!("Executing step: {}.{}", plugin, function);

        match runtime.call(plugin, function, &[]) {
            Ok(result) => {
                info!("Step completed: {:?}", result);
            }
            Err(e) => {
                error!("Step failed: {}", e);

                // Attempt rollback
                warn!("Initiating rollback");
                runtime.call("deploy", "rollback", &[])?;

                return Err(e);
            }
        }
    }

    info!("Orchestration completed successfully");
    Ok(())
}
```

## Custom Profiles

Define custom profiles for specialized use cases:

```rust
// Data processing profile
pub fn data_processing_capabilities() -> Capabilities {
    let mut caps = Capabilities::none();
    caps.grant(Capability::FileRead);
    caps.grant(Capability::FileWrite);
    caps.grant(Capability::TimeRead);
    // No network, no exec, limited to file I/O
    caps
}

// Web service profile
pub fn web_service_capabilities() -> Capabilities {
    let mut caps = Capabilities::safe_defaults();
    caps.grant(Capability::NetworkRequest);
    caps.grant(Capability::NetworkListen);
    caps.grant(Capability::EnvRead);
    // Limited file access, focus on network
    caps
}

// Testing profile
pub fn testing_capabilities() -> Capabilities {
    let mut caps = Capabilities::all();
    caps.set_strict_mode(true); // Log all operations
    caps.set_dry_run(true);     // Don't actually modify system
    caps
}
```

### Profile Registry

```rust
use std::collections::HashMap;

pub struct ProfileRegistry {
    profiles: HashMap<String, Capabilities>,
}

impl ProfileRegistry {
    pub fn new() -> Self {
        let mut profiles = HashMap::new();

        profiles.insert("terminal".to_string(), terminal_capabilities());
        profiles.insert("observability".to_string(), observability_capabilities());
        profiles.insert("orchestration".to_string(), orchestration_capabilities());
        profiles.insert("data-processing".to_string(), data_processing_capabilities());
        profiles.insert("web-service".to_string(), web_service_capabilities());

        Self { profiles }
    }

    pub fn get(&self, name: &str) -> Option<&Capabilities> {
        self.profiles.get(name)
    }

    pub fn register(&mut self, name: String, caps: Capabilities) {
        self.profiles.insert(name, caps);
    }

    pub fn list(&self) -> Vec<&str> {
        self.profiles.keys().map(|s| s.as_str()).collect()
    }
}
```

## See Also

- [Capabilities Guide](capabilities.md)
- [Runtime Guide](runtime-guide.md)
- [Manifest Schema](manifest-schema.md)
- [Multi-Process Safety](multi-process.md)
