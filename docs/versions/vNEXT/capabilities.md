# Capabilities System (vNEXT)

This document describes the capability system for secure plugin execution with fine-grained access control.

## Table of Contents

- [Overview](#overview)
- [Available Capabilities](#available-capabilities)
- [Capability Gating](#capability-gating)
- [Per-Host Allowlists](#per-host-allowlists)
- [Security Model](#security-model)
- [Examples](#examples)

## Overview

The capability system provides:
- **Fine-grained access control**: Plugins declare required capabilities
- **Least privilege principle**: Plugins only get what they need
- **Runtime enforcement**: Host validates all capability usage
- **Per-host allowlists**: Different hosts can grant different capabilities
- **Auditable access**: Track what plugins can do

### Capability Model

```
Plugin Manifest → Declares Required Capabilities
       ↓
Host Configuration → Defines Allowed Capabilities
       ↓
Runtime Validation → Grants intersection of required & allowed
       ↓
Plugin Execution → Enforces granted capabilities
```

## Available Capabilities

### Filesystem Capabilities

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `fs:read` | Read files and directories | Low |
| `fs:write` | Write and modify files | Medium |
| `fs:delete` | Delete files and directories | High |
| `fs:metadata` | Read file metadata (size, permissions) | Low |

Example manifest:
```toml
capabilities = ["fs:read", "fs:metadata"]
```

### Network Capabilities

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `net:request` | Make HTTP/HTTPS requests | Medium |
| `net:listen` | Listen on network sockets | High |
| `net:dns` | Perform DNS lookups | Low |

Example manifest:
```toml
capabilities = ["net:request", "net:dns"]
```

### System Capabilities

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `sys:env` | Read environment variables | Medium |
| `sys:exec` | Execute external processes | High |
| `sys:time` | Read system time | Low |

Example manifest:
```toml
capabilities = ["sys:time"]
```

### Inter-Plugin Capabilities

| Capability | Description | Risk Level |
|------------|-------------|------------|
| `plugin:call` | Call functions in other plugins | Medium |
| `plugin:message` | Send messages to other plugins | Low |

Example manifest:
```toml
capabilities = ["plugin:call"]
```

## Capability Gating

Capability gating restricts plugin access at multiple levels:

### 1. Manifest Declaration

Plugins must declare all required capabilities:

```toml
name = "file-processor"
version = "1.0.0"
capabilities = [
    "fs:read",
    "fs:write",
    "sys:time"
]
```

### 2. Host Configuration

Hosts define which capabilities they allow:

```rust
use fusabi_host::{Capabilities, Capability};

// Create empty capability set
let mut caps = Capabilities::none();

// Grant specific capabilities
caps.grant(Capability::FileRead);
caps.grant(Capability::FileWrite);
caps.grant(Capability::TimeRead);

// Use safe defaults (fs:read, fs:metadata, sys:time)
let safe_caps = Capabilities::safe_defaults();

// Grant all capabilities (NOT RECOMMENDED)
let all_caps = Capabilities::all();
```

### 3. Runtime Validation

The loader validates capability compatibility:

```rust
use fusabi_plugin_runtime::{PluginLoader, LoaderConfig};
use fusabi_host::{Capabilities, Capability, EngineConfig};

let mut caps = Capabilities::safe_defaults();
caps.grant(Capability::NetworkRequest);

let config = LoaderConfig::new()
    .with_engine_config(
        EngineConfig::default()
            .with_capabilities(caps)
    );

let loader = PluginLoader::new(config)?;

// This will succeed - plugin requires fs:read which is in safe_defaults
let plugin1 = loader.load_from_manifest("read-only.toml")?;

// This will fail - plugin requires fs:delete which isn't granted
let plugin2 = loader.load_from_manifest("destructive.toml");
assert!(plugin2.is_err());
```

### 4. Execution Enforcement

The runtime enforces capabilities during execution:

```rust
// Plugin with fs:read can read files
let content = plugin.call("read_file", &[Value::String("/data/input.txt".into())])?;

// Plugin without fs:write will fail at runtime
let result = plugin.call("write_file", &[
    Value::String("/data/output.txt".into()),
    Value::String("data".into())
]);
assert!(result.is_err()); // CapabilityDenied error
```

## Per-Host Allowlists

Different host types can define different capability allowlists:

### Terminal Host Profile

Interactive CLI tools with user oversight:

```rust
fn terminal_capabilities() -> Capabilities {
    let mut caps = Capabilities::safe_defaults();
    caps.grant(Capability::FileWrite);  // User can see file operations
    caps.grant(Capability::NetworkRequest);  // User can monitor network
    caps.grant(Capability::EnvRead);  // User context available
    caps
}
```

### Observability Host Profile

Monitoring and telemetry systems:

```rust
fn observability_capabilities() -> Capabilities {
    let mut caps = Capabilities::none();
    caps.grant(Capability::FileRead);  // Read logs
    caps.grant(Capability::FileMetadata);  // Check file sizes
    caps.grant(Capability::NetworkRequest);  // Send metrics
    caps.grant(Capability::TimeRead);  // Timestamp events
    // Explicitly NOT granted: fs:write, fs:delete, sys:exec
    caps
}
```

### Orchestration Host Profile

Workflow automation with elevated privileges:

```rust
fn orchestration_capabilities() -> Capabilities {
    let mut caps = Capabilities::all();
    // Orchestration needs broad access but we still validate
    caps
}
```

### Custom Host Profiles

Define custom capability sets:

```rust
fn data_processing_capabilities() -> Capabilities {
    let mut caps = Capabilities::none();
    caps.grant(Capability::FileRead);
    caps.grant(Capability::FileWrite);
    caps.grant(Capability::TimeRead);
    // No network, no exec, no delete
    caps
}

let config = LoaderConfig::new()
    .with_engine_config(
        EngineConfig::default()
            .with_capabilities(data_processing_capabilities())
    );
```

## Security Model

### Threat Model

The capability system defends against:

1. **Malicious plugins**: Plugins trying to access unauthorized resources
2. **Compromised plugins**: Plugins with vulnerabilities exploited by attackers
3. **Accidental damage**: Plugins with bugs that could harm the system
4. **Information disclosure**: Plugins accessing sensitive data

### Security Properties

- **Least Privilege**: Plugins only get necessary capabilities
- **Explicit Grants**: All access must be explicitly allowed
- **Runtime Enforcement**: Capabilities checked on every operation
- **Fail-Secure**: Unknown capabilities are denied by default
- **Auditable**: All capability usage is logged

### Security Best Practices

1. **Minimize granted capabilities**
   ```rust
   // Bad - grants everything
   let caps = Capabilities::all();

   // Good - grants only what's needed
   let mut caps = Capabilities::none();
   caps.grant(Capability::FileRead);
   ```

2. **Use host profiles appropriately**
   ```rust
   // Match capabilities to use case
   let caps = match host_type {
       HostType::Terminal => terminal_capabilities(),
       HostType::Observability => observability_capabilities(),
       HostType::Orchestration => orchestration_capabilities(),
   };
   ```

3. **Validate plugin manifests**
   ```rust
   let manifest = Manifest::from_file("plugin.toml")?;
   manifest.validate()?;

   // Check capabilities are reasonable
   for cap in &manifest.capabilities {
       if is_dangerous(cap) {
           eprintln!("Warning: plugin requires dangerous capability: {}", cap);
       }
   }
   ```

4. **Log capability denials**
   ```rust
   // Configure tracing to log denied operations
   use tracing::warn;

   if let Err(Error::CapabilityDenied(cap)) = result {
       warn!(
           plugin = plugin.name(),
           capability = cap,
           "Plugin attempted unauthorized operation"
       );
   }
   ```

5. **Review plugins before loading**
   ```rust
   fn review_plugin(manifest_path: &str) -> Result<()> {
       let manifest = Manifest::from_file(manifest_path)?;

       println!("Plugin: {}", manifest.name);
       println!("Version: {}", manifest.version);
       println!("Capabilities:");
       for cap in &manifest.capabilities {
           println!("  - {}", cap);
       }

       // Manual approval required for high-risk capabilities
       let high_risk = ["fs:delete", "sys:exec", "net:listen"];
       let has_high_risk = manifest.capabilities.iter()
           .any(|c| high_risk.contains(&c.as_str()));

       if has_high_risk {
           println!("WARNING: This plugin requires high-risk capabilities!");
           // Require explicit approval
       }

       Ok(())
   }
   ```

## Examples

### Example 1: Read-Only Data Plugin

```toml
# plugin.toml
name = "data-reader"
version = "1.0.0"
capabilities = ["fs:read", "fs:metadata"]
source = "reader.fsx"
```

```rust
// Load with minimal capabilities
let mut caps = Capabilities::none();
caps.grant(Capability::FileRead);
caps.grant(Capability::FileMetadata);

let config = LoaderConfig::new()
    .with_engine_config(
        EngineConfig::default().with_capabilities(caps)
    );

let loader = PluginLoader::new(config)?;
let plugin = loader.load_from_manifest("plugin.toml")?;
```

### Example 2: Network API Client

```toml
# api-client.toml
name = "api-client"
version = "1.0.0"
capabilities = ["net:request", "net:dns", "sys:time"]
source = "client.fsx"
```

```rust
let mut caps = Capabilities::safe_defaults(); // Includes sys:time
caps.grant(Capability::NetworkRequest);
caps.grant(Capability::DnsLookup);

let config = LoaderConfig::new()
    .with_engine_config(
        EngineConfig::default().with_capabilities(caps)
    );

let loader = PluginLoader::new(config)?;
let plugin = loader.load_from_manifest("api-client.toml")?;
```

### Example 3: Capability Checking

```rust
use fusabi_plugin_runtime::Manifest;

fn check_plugin_safety(manifest_path: &str) -> Result<bool> {
    let manifest = Manifest::from_file(manifest_path)?;

    // Define safe capabilities
    let safe = ["fs:read", "fs:metadata", "sys:time"];

    // Check if all required capabilities are safe
    let all_safe = manifest.capabilities.iter()
        .all(|cap| safe.contains(&cap.as_str()));

    if !all_safe {
        println!("Plugin requires elevated capabilities:");
        for cap in &manifest.capabilities {
            if !safe.contains(&cap.as_str()) {
                println!("  - {}", cap);
            }
        }
    }

    Ok(all_safe)
}
```

### Example 4: Dynamic Capability Granting

```rust
fn load_with_user_approval(manifest_path: &str) -> Result<Plugin> {
    let manifest = Manifest::from_file(manifest_path)?;

    let mut caps = Capabilities::none();

    // Request user approval for each capability
    for cap_name in &manifest.capabilities {
        println!("Plugin '{}' requests capability: {}", manifest.name, cap_name);
        print!("Grant? [y/N]: ");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "y" {
            if let Some(cap) = Capability::from_name(cap_name) {
                caps.grant(cap);
            }
        }
    }

    let config = LoaderConfig::new()
        .with_engine_config(
            EngineConfig::default().with_capabilities(caps)
        );

    let loader = PluginLoader::new(config)?;
    loader.load_from_manifest(manifest_path)
}
```

## See Also

- [Runtime Guide](runtime-guide.md)
- [Host Profiles](host-profiles.md)
- [Manifest Schema](manifest-schema.md)
- [Multi-Process Safety](multi-process.md)
