//! Error types for plugin runtime operations.

use thiserror::Error;

/// Result type alias using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during plugin operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Plugin not found.
    #[error("plugin not found: {0}")]
    PluginNotFound(String),

    /// Plugin already loaded.
    #[error("plugin already loaded: {0}")]
    PluginAlreadyLoaded(String),

    /// Invalid manifest.
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),

    /// Missing required field in manifest.
    #[error("missing required manifest field: {0}")]
    MissingManifestField(String),

    /// API version mismatch.
    #[error("API version mismatch: plugin requires {required}, host provides {provided}")]
    ApiVersionMismatch {
        /// Version required by plugin.
        required: String,
        /// Version provided by host.
        provided: String,
    },

    /// Missing required capability.
    #[error("missing required capability: {0}")]
    MissingCapability(String),

    /// Capability not declared in manifest.
    #[error("capability not declared in manifest: {0}")]
    UndeclaredCapability(String),

    /// Dependency not satisfied.
    #[error("dependency not satisfied: {name} requires {version}")]
    DependencyNotSatisfied {
        /// Dependency name.
        name: String,
        /// Required version.
        version: String,
    },

    /// Plugin initialization failed.
    #[error("plugin initialization failed: {0}")]
    InitializationFailed(String),

    /// Plugin execution failed.
    #[error("plugin execution failed: {0}")]
    ExecutionFailed(String),

    /// Plugin already in invalid state for operation.
    #[error("invalid plugin state: expected {expected}, got {actual}")]
    InvalidState {
        /// Expected state.
        expected: String,
        /// Actual state.
        actual: String,
    },

    /// Function not found in plugin.
    #[error("function not found: {0}")]
    FunctionNotFound(String),

    /// Compilation error.
    #[error("compilation error: {0}")]
    Compilation(String),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Host error.
    #[error("host error: {0}")]
    Host(#[from] fusabi_host::Error),

    /// Manifest parse error.
    #[cfg(feature = "serde")]
    #[error("manifest parse error: {0}")]
    ManifestParse(String),

    /// Watch error.
    #[cfg(feature = "watch")]
    #[error("watch error: {0}")]
    Watch(String),

    /// Plugin was unloaded.
    #[error("plugin was unloaded")]
    PluginUnloaded,

    /// Plugin reload failed.
    #[error("plugin reload failed: {0}")]
    ReloadFailed(String),

    /// Registry error.
    #[error("registry error: {0}")]
    Registry(String),
}

impl Error {
    /// Create a plugin not found error.
    pub fn plugin_not_found(name: impl Into<String>) -> Self {
        Self::PluginNotFound(name.into())
    }

    /// Create an invalid manifest error.
    pub fn invalid_manifest(msg: impl Into<String>) -> Self {
        Self::InvalidManifest(msg.into())
    }

    /// Create a missing manifest field error.
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingManifestField(field.into())
    }

    /// Create an API version mismatch error.
    pub fn api_version_mismatch(required: impl Into<String>, provided: impl Into<String>) -> Self {
        Self::ApiVersionMismatch {
            required: required.into(),
            provided: provided.into(),
        }
    }

    /// Create a missing capability error.
    pub fn missing_capability(cap: impl Into<String>) -> Self {
        Self::MissingCapability(cap.into())
    }

    /// Create a dependency not satisfied error.
    pub fn dependency_not_satisfied(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self::DependencyNotSatisfied {
            name: name.into(),
            version: version.into(),
        }
    }

    /// Create an initialization failed error.
    pub fn init_failed(msg: impl Into<String>) -> Self {
        Self::InitializationFailed(msg.into())
    }

    /// Create an execution failed error.
    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self::ExecutionFailed(msg.into())
    }

    /// Create an invalid state error.
    pub fn invalid_state(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::InvalidState {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Returns true if this error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::PluginNotFound(_)
                | Self::FunctionNotFound(_)
                | Self::InvalidState { .. }
        )
    }

    /// Returns true if this error should trigger a reload.
    pub fn should_reload(&self) -> bool {
        matches!(
            self,
            Self::Compilation(_) | Self::ExecutionFailed(_) | Self::ReloadFailed(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::plugin_not_found("my-plugin");
        assert_eq!(err.to_string(), "plugin not found: my-plugin");

        let err = Error::api_version_mismatch("0.2.0", "0.1.0");
        assert!(err.to_string().contains("0.2.0"));
        assert!(err.to_string().contains("0.1.0"));
    }

    #[test]
    fn test_error_classification() {
        assert!(Error::plugin_not_found("test").is_recoverable());
        assert!(!Error::init_failed("test").is_recoverable());

        assert!(Error::Compilation("test".into()).should_reload());
        assert!(!Error::plugin_not_found("test").should_reload());
    }
}
