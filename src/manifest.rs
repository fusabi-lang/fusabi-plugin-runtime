//! Plugin manifest schema and validation.

use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};

/// API version specification.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ApiVersion {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
}

impl ApiVersion {
    /// Create a new API version.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Parse from a string like "0.18.0".
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 2 {
            return Err(Error::invalid_manifest(format!("invalid version: {}", s)));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| Error::invalid_manifest(format!("invalid major version: {}", s)))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| Error::invalid_manifest(format!("invalid minor version: {}", s)))?;
        let patch = parts
            .get(2)
            .map(|p| p.parse().unwrap_or(0))
            .unwrap_or(0);

        Ok(Self { major, minor, patch })
    }

    /// Check if this version is compatible with another.
    pub fn is_compatible_with(&self, other: &ApiVersion) -> bool {
        // Same major version required, minor must be >= other
        self.major == other.major && self.minor >= other.minor
    }

    /// Format as a string.
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for ApiVersion {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 18,
            patch: 0,
        }
    }
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Plugin dependency specification.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dependency {
    /// Dependency name.
    pub name: String,
    /// Version requirement (semver).
    pub version: String,
    /// Whether this dependency is optional.
    #[cfg_attr(feature = "serde", serde(default))]
    pub optional: bool,
}

impl Dependency {
    /// Create a new required dependency.
    pub fn required(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            optional: false,
        }
    }

    /// Create a new optional dependency.
    pub fn optional(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            optional: true,
        }
    }
}

/// Plugin manifest defining metadata and requirements.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Manifest {
    /// Plugin name (unique identifier).
    pub name: String,

    /// Plugin version.
    pub version: String,

    /// Human-readable description.
    #[cfg_attr(feature = "serde", serde(default))]
    pub description: Option<String>,

    /// Plugin authors.
    #[cfg_attr(feature = "serde", serde(default))]
    pub authors: Vec<String>,

    /// Plugin license.
    #[cfg_attr(feature = "serde", serde(default))]
    pub license: Option<String>,

    /// Required Fusabi API version.
    #[cfg_attr(feature = "serde", serde(rename = "api-version"))]
    pub api_version: ApiVersion,

    /// Required capabilities.
    #[cfg_attr(feature = "serde", serde(default))]
    pub capabilities: Vec<String>,

    /// Plugin dependencies.
    #[cfg_attr(feature = "serde", serde(default))]
    pub dependencies: Vec<Dependency>,

    /// Entry point source file (.fsx).
    #[cfg_attr(feature = "serde", serde(default))]
    pub source: Option<String>,

    /// Pre-compiled bytecode file (.fzb).
    #[cfg_attr(feature = "serde", serde(default))]
    pub bytecode: Option<String>,

    /// Exported functions.
    #[cfg_attr(feature = "serde", serde(default))]
    pub exports: Vec<String>,

    /// Plugin tags for categorization.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tags: Vec<String>,

    /// Custom metadata.
    #[cfg_attr(feature = "serde", serde(default))]
    pub metadata: HashMap<String, String>,
}

impl Manifest {
    /// Create a new manifest with required fields.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: None,
            authors: Vec::new(),
            license: None,
            api_version: ApiVersion::default(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            source: None,
            bytecode: None,
            exports: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Load manifest from a TOML file.
    #[cfg(feature = "serde")]
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }

    /// Parse manifest from TOML string.
    #[cfg(feature = "serde")]
    pub fn from_toml(content: &str) -> Result<Self> {
        toml::from_str(content).map_err(|e| Error::ManifestParse(e.to_string()))
    }

    /// Parse manifest from JSON string.
    #[cfg(feature = "serde")]
    pub fn from_json(content: &str) -> Result<Self> {
        serde_json::from_str(content).map_err(|e| Error::ManifestParse(e.to_string()))
    }

    /// Serialize to TOML string.
    #[cfg(feature = "serde")]
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).map_err(|e| Error::ManifestParse(e.to_string()))
    }

    /// Serialize to JSON string.
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| Error::ManifestParse(e.to_string()))
    }

    /// Validate the manifest.
    pub fn validate(&self) -> Result<()> {
        // Check required fields
        if self.name.is_empty() {
            return Err(Error::missing_field("name"));
        }

        if self.version.is_empty() {
            return Err(Error::missing_field("version"));
        }

        // Must have either source or bytecode
        if self.source.is_none() && self.bytecode.is_none() {
            return Err(Error::invalid_manifest(
                "manifest must specify either 'source' or 'bytecode'",
            ));
        }

        // Validate capability names
        for cap in &self.capabilities {
            if fusabi_host::Capability::from_name(cap).is_none() {
                return Err(Error::invalid_manifest(format!(
                    "unknown capability: {}",
                    cap
                )));
            }
        }

        Ok(())
    }

    /// Check if this manifest requires a capability.
    pub fn requires_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }

    /// Check if this manifest is compatible with a host API version.
    pub fn is_compatible_with_host(&self, host_version: &ApiVersion) -> bool {
        host_version.is_compatible_with(&self.api_version)
    }

    /// Get the entry point path (source or bytecode).
    pub fn entry_point(&self) -> Option<&str> {
        self.source.as_deref().or(self.bytecode.as_deref())
    }

    /// Check if using source code (vs pre-compiled bytecode).
    pub fn uses_source(&self) -> bool {
        self.source.is_some()
    }
}

/// Builder for creating manifests.
pub struct ManifestBuilder {
    manifest: Manifest,
}

impl ManifestBuilder {
    /// Create a new manifest builder.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            manifest: Manifest::new(name, version),
        }
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.manifest.description = Some(desc.into());
        self
    }

    /// Add an author.
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.manifest.authors.push(author.into());
        self
    }

    /// Set the license.
    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.manifest.license = Some(license.into());
        self
    }

    /// Set the API version.
    pub fn api_version(mut self, version: ApiVersion) -> Self {
        self.manifest.api_version = version;
        self
    }

    /// Add a capability requirement.
    pub fn capability(mut self, cap: impl Into<String>) -> Self {
        self.manifest.capabilities.push(cap.into());
        self
    }

    /// Add capabilities.
    pub fn capabilities<I, S>(mut self, caps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.manifest.capabilities.extend(caps.into_iter().map(Into::into));
        self
    }

    /// Add a dependency.
    pub fn dependency(mut self, dep: Dependency) -> Self {
        self.manifest.dependencies.push(dep);
        self
    }

    /// Set the source file.
    pub fn source(mut self, path: impl Into<String>) -> Self {
        self.manifest.source = Some(path.into());
        self
    }

    /// Set the bytecode file.
    pub fn bytecode(mut self, path: impl Into<String>) -> Self {
        self.manifest.bytecode = Some(path.into());
        self
    }

    /// Add an export.
    pub fn export(mut self, name: impl Into<String>) -> Self {
        self.manifest.exports.push(name.into());
        self
    }

    /// Add exports.
    pub fn exports<I, S>(mut self, exports: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.manifest.exports.extend(exports.into_iter().map(Into::into));
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.manifest.tags.push(tag.into());
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.manifest.metadata.insert(key.into(), value.into());
        self
    }

    /// Build and validate the manifest.
    pub fn build(self) -> Result<Manifest> {
        self.manifest.validate()?;
        Ok(self.manifest)
    }

    /// Build without validation.
    pub fn build_unchecked(self) -> Manifest {
        self.manifest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_parse() {
        let v = ApiVersion::parse("0.18.5").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 18);
        assert_eq!(v.patch, 5);

        let v = ApiVersion::parse("1.0").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_api_version_compatibility() {
        let v1 = ApiVersion::new(0, 18, 0);
        let v2 = ApiVersion::new(0, 18, 5);
        let v3 = ApiVersion::new(0, 19, 0);
        let v4 = ApiVersion::new(1, 0, 0);

        // Same version compatible
        assert!(v1.is_compatible_with(&v1));

        // Higher patch compatible
        assert!(v2.is_compatible_with(&v1));

        // Higher minor compatible
        assert!(v3.is_compatible_with(&v1));

        // Lower minor not compatible
        assert!(!v1.is_compatible_with(&v3));

        // Different major not compatible
        assert!(!v4.is_compatible_with(&v1));
    }

    #[test]
    fn test_manifest_builder() {
        let manifest = ManifestBuilder::new("test-plugin", "1.0.0")
            .description("A test plugin")
            .author("Test Author")
            .license("MIT")
            .capability("fs:read")
            .capability("net:request")
            .source("plugin.fsx")
            .export("main")
            .tag("test")
            .build()
            .unwrap();

        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.capabilities.len(), 2);
        assert!(manifest.requires_capability("fs:read"));
    }

    #[test]
    fn test_manifest_validation() {
        // Missing name
        let manifest = Manifest {
            name: String::new(),
            version: "1.0.0".into(),
            ..Manifest::new("", "1.0.0")
        };
        assert!(manifest.validate().is_err());

        // Missing entry point
        let manifest = Manifest::new("test", "1.0.0");
        assert!(manifest.validate().is_err());

        // Invalid capability
        let mut manifest = Manifest::new("test", "1.0.0");
        manifest.source = Some("test.fsx".into());
        manifest.capabilities.push("invalid:cap".into());
        assert!(manifest.validate().is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_manifest_toml() {
        let toml = r#"
name = "my-plugin"
version = "1.0.0"
description = "A sample plugin"
api-version = { major = 0, minor = 18, patch = 0 }
capabilities = ["fs:read", "time:read"]
source = "main.fsx"
exports = ["init", "run"]
"#;

        let manifest = Manifest::from_toml(toml).unwrap();
        assert_eq!(manifest.name, "my-plugin");
        assert_eq!(manifest.capabilities.len(), 2);
    }
}
