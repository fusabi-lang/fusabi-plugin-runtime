# Documentation Structure

This document describes the required structure and sections for fusabi-plugin-runtime documentation.

## Directory Layout

```
docs/
├── STRUCTURE.md           # This file - describes documentation organization
├── RELEASE.md             # Release process and workflow
└── versions/              # Versioned documentation
    └── vNEXT/             # Next version (unreleased)
        ├── runtime-guide.md        # Runtime usage guide
        ├── capabilities.md         # Capability system documentation
        ├── host-profiles.md        # Host profile specifications
        ├── manifest-schema.md      # Plugin manifest schema reference
        ├── hot-reload.md           # Hot reload implementation details
        ├── multi-process.md        # Multi-process safety guide
        └── migration.md            # Migration guide from previous versions
```

## Required Sections

### Runtime Guide (`runtime-guide.md`)
- **Purpose**: Comprehensive guide for using the plugin runtime
- **Required sections**:
  - Quick Start
  - Plugin Loading (source, bytecode, manifest)
  - Plugin Registry Management
  - Lifecycle Hooks
  - Error Handling
  - Configuration Options
  - Examples

### Capabilities (`capabilities.md`)
- **Purpose**: Document the capability system and enforcement
- **Required sections**:
  - Overview of capability model
  - Available capabilities
  - Capability gating and allowlists
  - Per-host capability configuration
  - Security considerations
  - Examples

### Host Profiles (`host-profiles.md`)
- **Purpose**: Define host profiles and their characteristics
- **Required sections**:
  - Profile types (terminal, observability, orchestration)
  - Capability sets per profile
  - Plugin selection criteria
  - Integration patterns
  - Examples for each profile

### Manifest Schema (`manifest-schema.md`)
- **Purpose**: Complete reference for plugin manifests
- **Required sections**:
  - Schema specification (TOML/JSON)
  - Required fields
  - Optional fields
  - UI plugin vs daemon plugin differences
  - Validation rules
  - Examples

### Hot Reload (`hot-reload.md`)
- **Purpose**: Document hot reload functionality
- **Required sections**:
  - Watcher configuration
  - Debounce and backoff strategies
  - Event handling
  - State preservation
  - Error recovery
  - Examples

### Multi-Process Safety (`multi-process.md`)
- **Purpose**: Guide for multi-process plugin execution
- **Required sections**:
  - Sandboxing architecture
  - Inter-process communication
  - Resource isolation
  - Concurrent access patterns
  - Safety guarantees
  - Examples

### Migration Guide (`migration.md`)
- **Purpose**: Help users migrate between versions
- **Required sections**:
  - Breaking changes
  - Deprecated features
  - New features
  - Code migration examples
  - Troubleshooting

## Versioning Strategy

1. **vNEXT**: Contains unreleased documentation for the next version
2. **v0.1.x**: Documentation for 0.1.x releases (created on release)
3. **v0.2.x**: Documentation for 0.2.x releases (future)

When releasing:
1. Copy `vNEXT/` to `v{VERSION}/`
2. Update version references in copied docs
3. Keep `vNEXT/` for ongoing development

## Documentation Standards

### Writing Style
- Use clear, concise language
- Include code examples for all features
- Provide both simple and advanced use cases
- Document error conditions and recovery

### Code Examples
- Must be valid Rust code
- Include necessary imports
- Show complete, runnable examples when possible
- Use comments to explain key concepts

### Formatting
- Use Markdown formatting consistently
- Include table of contents for long documents
- Use code fences with language specifiers
- Link to related documentation

## Documentation Checks

The CI pipeline validates:
- All required files exist
- Markdown syntax is valid
- Code examples compile (when possible)
- Internal links are not broken
- Version consistency

See `.github/workflows/ci.yml` for implementation details.

## Maintenance

- Update documentation with code changes in the same PR
- Review documentation quarterly for accuracy
- Archive old version docs when no longer supported
- Keep STRUCTURE.md updated as requirements evolve
