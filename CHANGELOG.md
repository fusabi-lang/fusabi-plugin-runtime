# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-12-04

### Added
- Initial release of `fusabi-plugin-runtime`.
- Added `PluginLoader` for loading `.fsx` and `.fzb` plugins.
- Added manifest validation (`plugin.toml`) with capability requirements.
- Added hot-reload support (via `watch` feature).
- Added lifecycle hooks and metrics integration.
