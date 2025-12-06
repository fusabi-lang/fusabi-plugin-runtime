# Manifest Schema (vNEXT)

Complete reference for plugin manifest files.

## Table of Contents

- [Overview](#overview)
- [Schema Specification](#schema-specification)
- [Required Fields](#required-fields)
- [Optional Fields](#optional-fields)
- [UI vs Daemon Plugins](#ui-vs-daemon-plugins)
- [Validation Rules](#validation-rules)
- [Examples](#examples)

## Overview

Plugin manifests define metadata, capabilities, and requirements. Manifests can be written in TOML or JSON format.

### Basic Structure

```toml
# Required metadata
name = "plugin-name"
version = "1.0.0"
api-version = { major = 0, minor = 18, patch = 0 }

# Optional metadata
description = "Plugin description"
authors = ["Author Name <author@example.com>"]
license = "MIT OR Apache-2.0"

# Capabilities and dependencies
capabilities = ["fs:read", "net:request"]
dependencies = [
    { name = "json", version = "^1.0" }
]

# Entry point
source = "main.fsx"
exports = ["init", "process", "cleanup"]

# Categorization
tags = ["utility", "data-processing"]

# Custom metadata
[metadata]
category = "ui"
interactive = true
```

## Schema Specification

### TOML Schema

```toml
# ===== REQUIRED FIELDS =====

# Plugin identifier (unique name)
name = "string"

# Semantic version (semver)
version = "string"

# Required Fusabi API version
[api-version]
major = integer
minor = integer
patch = integer

# ===== OPTIONAL FIELDS =====

# Human-readable description
description = "string"

# Plugin authors
authors = ["string", ...]

# SPDX license identifier
license = "string"

# Required capabilities (array of strings)
capabilities = ["string", ...]

# Plugin dependencies
[[dependencies]]
name = "string"
version = "string"       # Semver requirement
optional = boolean       # Default: false

# Entry point (source OR bytecode required)
source = "path/to/file.fsx"
bytecode = "path/to/file.fzb"

# Exported functions
exports = ["string", ...]

# Categorization tags
tags = ["string", ...]

# Custom metadata (key-value pairs)
[metadata]
key = "value"
```

### JSON Schema

```json
{
  "name": "string (required)",
  "version": "string (required)",
  "api-version": {
    "major": "integer (required)",
    "minor": "integer (required)",
    "patch": "integer (required)"
  },
  "description": "string (optional)",
  "authors": ["string", "..."],
  "license": "string (optional)",
  "capabilities": ["string", "..."],
  "dependencies": [
    {
      "name": "string",
      "version": "string",
      "optional": "boolean (default: false)"
    }
  ],
  "source": "string (optional)",
  "bytecode": "string (optional)",
  "exports": ["string", "..."],
  "tags": ["string", "..."],
  "metadata": {
    "key": "value"
  }
}
```

## Required Fields

### `name`

**Type**: String
**Description**: Unique identifier for the plugin
**Format**: Lowercase, alphanumeric, hyphens allowed
**Example**: `"my-plugin"`, `"data-processor"`

```toml
name = "file-converter"
```

**Validation**:
- Must not be empty
- Must match pattern: `^[a-z0-9][a-z0-9-]*$`
- Should be unique in the plugin registry

### `version`

**Type**: String
**Description**: Plugin version following semantic versioning
**Format**: `MAJOR.MINOR.PATCH`
**Example**: `"1.0.0"`, `"0.2.1-beta"`

```toml
version = "1.0.0"
```

**Validation**:
- Must be valid semver format
- Must not be empty

### `api-version`

**Type**: Object
**Description**: Required Fusabi API version
**Fields**: `major`, `minor`, `patch` (all integers)

```toml
api-version = { major = 0, minor = 18, patch = 0 }
```

**Compatibility**: Host API version must be compatible (same major, minor >= plugin's minor)

## Optional Fields

### `description`

**Type**: String
**Description**: Human-readable plugin description
**Example**: `"Converts files between different formats"`

```toml
description = "A plugin for processing CSV files"
```

### `authors`

**Type**: Array of strings
**Description**: Plugin authors
**Format**: `"Name <email>"` or `"Name"`

```toml
authors = [
    "Jane Doe <jane@example.com>",
    "John Smith"
]
```

### `license`

**Type**: String
**Description**: SPDX license identifier
**Example**: `"MIT"`, `"Apache-2.0"`, `"MIT OR Apache-2.0"`

```toml
license = "MIT OR Apache-2.0"
```

### `capabilities`

**Type**: Array of strings
**Description**: Required capabilities
**Valid values**: See [Capabilities Guide](capabilities.md)

```toml
capabilities = [
    "fs:read",
    "fs:write",
    "net:request",
    "sys:time"
]
```

**Default**: `[]` (no capabilities)

### `dependencies`

**Type**: Array of dependency objects
**Description**: Plugin dependencies
**Fields**:
- `name`: Dependency name (string, required)
- `version`: Version requirement (string, required)
- `optional`: Whether dependency is optional (boolean, default: false)

```toml
[[dependencies]]
name = "json"
version = "^1.0"

[[dependencies]]
name = "xml"
version = ">=2.0.0"
optional = true
```

**Version syntax**:
- `"1.0.0"` - Exact version
- `"^1.0"` - Compatible (1.x)
- `"~1.2"` - Patch updates (1.2.x)
- `">=1.0.0"` - Greater or equal

### `source`

**Type**: String
**Description**: Path to source file (.fsx)
**Note**: Either `source` OR `bytecode` must be specified

```toml
source = "main.fsx"
```

**Relative paths**: Resolved relative to manifest directory

### `bytecode`

**Type**: String
**Description**: Path to bytecode file (.fzb)
**Note**: Either `source` OR `bytecode` must be specified

```toml
bytecode = "plugin.fzb"
```

### `exports`

**Type**: Array of strings
**Description**: Functions exported by plugin
**Example**: `["init", "process", "cleanup"]`

```toml
exports = ["init", "run", "stop"]
```

**Default**: `[]`

### `tags`

**Type**: Array of strings
**Description**: Categorization tags
**Example**: `["utility", "data-processing"]`

```toml
tags = ["cli", "converter", "data"]
```

**Default**: `[]`

### `metadata`

**Type**: Object (key-value pairs)
**Description**: Custom metadata
**Note**: All values must be strings

```toml
[metadata]
category = "ui"
interactive = "true"
homepage = "https://example.com"
```

**Default**: `{}`

## UI vs Daemon Plugins

### UI Plugin Manifest

UI plugins are interactive and user-facing:

```toml
name = "file-browser"
version = "1.0.0"
description = "Interactive file browser"
api-version = { major = 0, minor = 18, patch = 0 }

capabilities = [
    "fs:read",
    "fs:metadata"
]

source = "browser.fsx"
exports = ["init", "render", "handle_input", "cleanup"]

tags = ["ui", "interactive", "browser"]

[metadata]
category = "ui"
interactive = "true"
requires-terminal = "true"
```

**Characteristics**:
- `category = "ui"` in metadata
- `interactive = "true"` in metadata
- Exports may include: `render`, `handle_input`, `on_key`, `on_mouse`
- Typically requires terminal or GUI capabilities
- User-triggered execution

### Daemon Plugin Manifest

Daemon plugins run in the background:

```toml
name = "metrics-collector"
version = "1.0.0"
description = "System metrics collector"
api-version = { major = 0, minor = 18, patch = 0 }

capabilities = [
    "fs:read",
    "net:request",
    "sys:time"
]

source = "collector.fsx"
exports = ["init", "collect", "report", "shutdown"]

tags = ["daemon", "monitoring", "metrics"]

[metadata]
category = "daemon"
interactive = "false"
run-mode = "background"
interval = "60"  # Collection interval in seconds
```

**Characteristics**:
- `category = "daemon"` in metadata
- `interactive = "false"` in metadata
- `run-mode = "background"` in metadata
- May specify `interval` for periodic execution
- Exports may include: `collect`, `report`, `health_check`
- Autonomous execution

### Hybrid Plugins

Some plugins support both modes:

```toml
name = "log-viewer"
version = "1.0.0"
description = "Log file viewer and analyzer"
api-version = { major = 0, minor = 18, patch = 0 }

capabilities = ["fs:read", "fs:metadata"]

source = "viewer.fsx"
exports = [
    "init",
    "view",          # UI mode
    "analyze",       # Daemon mode
    "cleanup"
]

tags = ["logs", "monitoring", "ui", "daemon"]

[metadata]
category = "hybrid"
supports-ui = "true"
supports-daemon = "true"
```

## Validation Rules

### Name Validation

```rust
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err("name cannot be empty");
    }

    let pattern = Regex::new(r"^[a-z0-9][a-z0-9-]*$")?;
    if !pattern.is_match(name) {
        return Err("name must be lowercase alphanumeric with hyphens");
    }

    Ok(())
}
```

### Version Validation

```rust
fn validate_version(version: &str) -> Result<()> {
    semver::Version::parse(version)
        .map_err(|_| "invalid semantic version")?;
    Ok(())
}
```

### Entry Point Validation

```rust
fn validate_entry_point(manifest: &Manifest) -> Result<()> {
    if manifest.source.is_none() && manifest.bytecode.is_none() {
        return Err("must specify either source or bytecode");
    }

    if manifest.source.is_some() && manifest.bytecode.is_some() {
        return Err("cannot specify both source and bytecode");
    }

    Ok(())
}
```

### Capability Validation

```rust
fn validate_capabilities(capabilities: &[String]) -> Result<()> {
    for cap in capabilities {
        if Capability::from_name(cap).is_none() {
            return Err(format!("unknown capability: {}", cap));
        }
    }
    Ok(())
}
```

### API Version Compatibility

```rust
fn validate_api_compatibility(
    plugin_version: &ApiVersion,
    host_version: &ApiVersion
) -> Result<()> {
    if !host_version.is_compatible_with(plugin_version) {
        return Err(format!(
            "incompatible API version: plugin requires {}, host is {}",
            plugin_version, host_version
        ));
    }
    Ok(())
}
```

## Examples

### Example 1: Minimal Plugin

```toml
name = "hello"
version = "1.0.0"
api-version = { major = 0, minor = 18, patch = 0 }
source = "hello.fsx"
```

### Example 2: Full-Featured Plugin

```toml
name = "data-processor"
version = "2.1.0"
description = "Advanced data processing plugin"
authors = ["Data Team <data@example.com>"]
license = "MIT"

api-version = { major = 0, minor = 18, patch = 0 }

capabilities = [
    "fs:read",
    "fs:write",
    "net:request",
    "sys:time"
]

[[dependencies]]
name = "json"
version = "^1.0"

[[dependencies]]
name = "csv"
version = "^2.0"

[[dependencies]]
name = "xml"
version = ">=1.5.0"
optional = true

source = "processor.fsx"

exports = [
    "init",
    "process_json",
    "process_csv",
    "process_xml",
    "cleanup"
]

tags = ["data", "processing", "converter"]

[metadata]
category = "daemon"
homepage = "https://github.com/example/data-processor"
documentation = "https://docs.example.com/data-processor"
```

### Example 3: UI Plugin

```toml
name = "task-manager"
version = "1.0.0"
description = "Interactive task manager interface"
api-version = { major = 0, minor = 18, patch = 0 }

capabilities = ["fs:read", "fs:write"]

source = "manager.fsx"
exports = ["init", "render", "on_key", "on_resize", "cleanup"]

tags = ["ui", "tasks", "productivity"]

[metadata]
category = "ui"
interactive = "true"
requires-terminal = "true"
min-width = "80"
min-height = "24"
```

### Example 4: JSON Format

```json
{
  "name": "api-client",
  "version": "1.5.0",
  "description": "REST API client",
  "authors": ["API Team"],
  "license": "Apache-2.0",
  "api-version": {
    "major": 0,
    "minor": 18,
    "patch": 0
  },
  "capabilities": [
    "net:request",
    "net:dns",
    "sys:time"
  ],
  "dependencies": [
    {
      "name": "json",
      "version": "^1.0"
    }
  ],
  "source": "client.fsx",
  "exports": ["init", "get", "post", "put", "delete"],
  "tags": ["api", "network", "client"],
  "metadata": {
    "category": "daemon",
    "api-version": "v1"
  }
}
```

## See Also

- [Runtime Guide](runtime-guide.md)
- [Capabilities Guide](capabilities.md)
- [Host Profiles](host-profiles.md)
