//! Plugin loading and compilation.

use std::path::{Path, PathBuf};

use fusabi_host::{
    compile::{compile_source, compile_file, CompileOptions},
    EngineConfig, Capabilities, Limits,
};

use crate::error::{Error, Result};
use crate::manifest::{ApiVersion, Manifest};
use crate::plugin::{Plugin, PluginHandle};

/// Configuration for the plugin loader.
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Default engine configuration for plugins.
    pub engine_config: EngineConfig,
    /// Compilation options.
    pub compile_options: CompileOptions,
    /// Host API version.
    pub host_api_version: ApiVersion,
    /// Base path for resolving relative paths.
    pub base_path: Option<PathBuf>,
    /// Whether to automatically start plugins after loading.
    pub auto_start: bool,
    /// Whether to validate manifests strictly.
    pub strict_validation: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            engine_config: EngineConfig::default(),
            compile_options: CompileOptions::default(),
            host_api_version: ApiVersion::default(),
            base_path: None,
            auto_start: true,
            strict_validation: true,
        }
    }
}

impl LoaderConfig {
    /// Create a new loader configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the engine configuration.
    pub fn with_engine_config(mut self, config: EngineConfig) -> Self {
        self.engine_config = config;
        self
    }

    /// Set the compile options.
    pub fn with_compile_options(mut self, options: CompileOptions) -> Self {
        self.compile_options = options;
        self
    }

    /// Set the host API version.
    pub fn with_host_api_version(mut self, version: ApiVersion) -> Self {
        self.host_api_version = version;
        self
    }

    /// Set the base path.
    pub fn with_base_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.base_path = Some(path.into());
        self
    }

    /// Set auto-start behavior.
    pub fn with_auto_start(mut self, auto_start: bool) -> Self {
        self.auto_start = auto_start;
        self
    }

    /// Set strict validation.
    pub fn with_strict_validation(mut self, strict: bool) -> Self {
        self.strict_validation = strict;
        self
    }

    /// Create a strict loader config.
    pub fn strict() -> Self {
        Self {
            engine_config: EngineConfig::strict(),
            compile_options: CompileOptions::production(),
            host_api_version: ApiVersion::default(),
            base_path: None,
            auto_start: false,
            strict_validation: true,
        }
    }
}

/// Plugin loader for loading plugins from manifests and source files.
pub struct PluginLoader {
    config: LoaderConfig,
}

impl PluginLoader {
    /// Create a new plugin loader.
    pub fn new(config: LoaderConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Get the loader configuration.
    pub fn config(&self) -> &LoaderConfig {
        &self.config
    }

    /// Load a plugin from a manifest file.
    #[cfg(feature = "serde")]
    pub fn load_from_manifest(&self, manifest_path: impl AsRef<Path>) -> Result<PluginHandle> {
        let manifest_path = self.resolve_path(manifest_path.as_ref());
        let manifest = Manifest::from_file(&manifest_path)?;

        self.load_manifest(manifest, Some(manifest_path))
    }

    /// Load a plugin from a manifest object.
    pub fn load_manifest(
        &self,
        manifest: Manifest,
        manifest_path: Option<PathBuf>,
    ) -> Result<PluginHandle> {
        // Validate manifest
        if self.config.strict_validation {
            manifest.validate()?;
        }

        // Check API version compatibility
        if !manifest.is_compatible_with_host(&self.config.host_api_version) {
            return Err(Error::api_version_mismatch(
                manifest.api_version.to_string(),
                self.config.host_api_version.to_string(),
            ));
        }

        // Create plugin
        let plugin = Plugin::new(manifest.clone());

        // Resolve entry point path
        let entry_path = manifest.entry_point().map(|p| {
            if let Some(ref manifest_path) = manifest_path {
                manifest_path.parent().unwrap_or(Path::new(".")).join(p)
            } else {
                self.resolve_path(Path::new(p))
            }
        });

        // Load source or bytecode
        if let Some(ref entry_path) = entry_path {
            if manifest.uses_source() {
                self.compile_and_load(&plugin, entry_path)?;
            } else {
                self.load_bytecode(&plugin, entry_path)?;
            }
        }

        // Build engine config with required capabilities
        let engine_config = self.build_engine_config(&manifest)?;

        // Initialize plugin
        plugin.initialize(engine_config)?;

        // Auto-start if configured
        if self.config.auto_start {
            plugin.start()?;
        }

        Ok(PluginHandle::new(plugin))
    }

    /// Load a plugin from a source file directly.
    pub fn load_source(&self, source_path: impl AsRef<Path>) -> Result<PluginHandle> {
        let source_path = self.resolve_path(source_path.as_ref());

        // Read and parse source for embedded manifest
        let source = std::fs::read_to_string(&source_path)?;

        // Create a minimal manifest
        let name = source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        let manifest = Manifest::new(name, "0.0.0");

        // Create plugin
        let plugin = Plugin::new(manifest);

        // Compile source
        let compile_result = compile_source(&source, &self.config.compile_options)?;
        plugin.set_bytecode(compile_result.bytecode);

        // Initialize with default config
        plugin.initialize(self.config.engine_config.clone())?;

        // Auto-start if configured
        if self.config.auto_start {
            plugin.start()?;
        }

        Ok(PluginHandle::new(plugin))
    }

    /// Load a plugin from bytecode directly.
    pub fn load_bytecode_file(&self, bytecode_path: impl AsRef<Path>) -> Result<PluginHandle> {
        let bytecode_path = self.resolve_path(bytecode_path.as_ref());

        // Read bytecode
        let bytecode = std::fs::read(&bytecode_path)?;

        // Validate bytecode
        let metadata = fusabi_host::compile::validate_bytecode(&bytecode)?;

        // Create manifest from bytecode metadata
        let name = bytecode_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        let manifest = Manifest::new(name, metadata.compiler_version.clone());

        // Create plugin
        let plugin = Plugin::new(manifest);
        plugin.set_bytecode(bytecode);

        // Initialize with default config
        plugin.initialize(self.config.engine_config.clone())?;

        // Auto-start if configured
        if self.config.auto_start {
            plugin.start()?;
        }

        Ok(PluginHandle::new(plugin))
    }

    /// Reload a plugin.
    pub fn reload(&self, plugin: &PluginHandle) -> Result<()> {
        plugin.inner().reload()
    }

    // Helper methods

    fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(ref base) = self.config.base_path {
            base.join(path)
        } else {
            path.to_path_buf()
        }
    }

    fn compile_and_load(&self, plugin: &Plugin, source_path: &Path) -> Result<()> {
        let compile_result = compile_file(source_path, &self.config.compile_options)
            .map_err(|e| Error::Compilation(e.to_string()))?;

        plugin.set_bytecode(compile_result.bytecode);

        // Log warnings
        for warning in &compile_result.warnings {
            tracing::warn!("Plugin {}: {}", plugin.name(), warning.message);
        }

        Ok(())
    }

    fn load_bytecode(&self, plugin: &Plugin, bytecode_path: &Path) -> Result<()> {
        let bytecode = std::fs::read(bytecode_path)?;

        // Validate
        fusabi_host::compile::validate_bytecode(&bytecode)?;

        plugin.set_bytecode(bytecode);
        Ok(())
    }

    fn build_engine_config(&self, manifest: &Manifest) -> Result<EngineConfig> {
        // Start with base config
        let mut config = self.config.engine_config.clone();

        // Add required capabilities
        let mut caps = config.capabilities.clone();
        for cap_name in &manifest.capabilities {
            let cap = fusabi_host::Capability::from_name(cap_name)
                .ok_or_else(|| Error::invalid_manifest(format!("unknown capability: {}", cap_name)))?;
            caps.grant(cap);
        }
        config.capabilities = caps;

        Ok(config)
    }
}

impl std::fmt::Debug for PluginLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginLoader")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ManifestBuilder;

    #[test]
    fn test_loader_config_builder() {
        let config = LoaderConfig::new()
            .with_auto_start(false)
            .with_strict_validation(true);

        assert!(!config.auto_start);
        assert!(config.strict_validation);
    }

    #[test]
    fn test_loader_creation() {
        let loader = PluginLoader::new(LoaderConfig::default()).unwrap();
        assert!(loader.config().auto_start);
    }

    #[test]
    fn test_load_manifest() {
        let loader = PluginLoader::new(
            LoaderConfig::new().with_auto_start(false),
        )
        .unwrap();

        let manifest = ManifestBuilder::new("test-plugin", "1.0.0")
            .source("test.fsx")
            .build_unchecked();

        // This will fail because the source file doesn't exist,
        // but it tests the loading logic
        let result = loader.load_manifest(manifest, None);

        // Should fail on missing source file
        assert!(result.is_err());
    }

    #[test]
    fn test_api_version_check() {
        let loader = PluginLoader::new(
            LoaderConfig::new()
                .with_host_api_version(ApiVersion::new(0, 18, 0))
                .with_auto_start(false),
        )
        .unwrap();

        // Plugin requiring newer version should fail
        let manifest = ManifestBuilder::new("test", "1.0.0")
            .api_version(ApiVersion::new(1, 0, 0))
            .source("test.fsx")
            .build_unchecked();

        let result = loader.load_manifest(manifest, None);
        assert!(matches!(result, Err(Error::ApiVersionMismatch { .. })));
    }
}
