//! Example demonstrating plugin loading and execution.

use fusabi_plugin_runtime::{
    loader::{LoaderConfig, PluginLoader},
    manifest::{ApiVersion, ManifestBuilder},
    registry::{PluginRegistry, RegistryConfig},
    Capabilities, Limits, Value,
};

fn main() -> fusabi_plugin_runtime::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Plugin Loader Example ===\n");

    // Create a plugin manifest programmatically
    let manifest = ManifestBuilder::new("example-plugin", "1.0.0")
        .description("An example plugin demonstrating the loader")
        .author("Example Author")
        .api_version(ApiVersion::new(0, 18, 0))
        .capability("time:read")
        .capability("logging")
        .source("example.fsx") // Would need to exist for actual loading
        .export("init")
        .export("process")
        .export("cleanup")
        .tag("example")
        .metadata("category", "demo")
        .build_unchecked(); // Use build() for validation

    println!("Created manifest for: {}", manifest.name);
    println!("  Version: {}", manifest.version);
    println!("  API Version: {}", manifest.api_version);
    println!("  Capabilities: {:?}", manifest.capabilities);
    println!("  Exports: {:?}", manifest.exports);

    // Demonstrate loader configuration
    println!("\n=== Loader Configuration ===");

    let loader_config = LoaderConfig::new()
        .with_host_api_version(ApiVersion::new(0, 18, 0))
        .with_auto_start(false)
        .with_strict_validation(true);

    println!("Loader config:");
    println!("  Host API: {}", loader_config.host_api_version);
    println!("  Auto start: {}", loader_config.auto_start);
    println!("  Strict validation: {}", loader_config.strict_validation);

    // Create loader
    let loader = PluginLoader::new(loader_config)?;

    // Note: In a real scenario, you would load from an actual file:
    // let plugin = loader.load_from_manifest("plugin.toml")?;

    // Demonstrate registry usage
    println!("\n=== Plugin Registry ===");

    let registry_config = RegistryConfig::new()
        .with_max_plugins(10)
        .with_allow_overwrite(false);

    let registry = PluginRegistry::new(registry_config);

    println!("Registry config:");
    println!("  Max plugins: {}", registry.config().max_plugins);
    println!("  Allow overwrite: {}", registry.config().allow_overwrite);

    println!("\nRegistry stats:");
    let stats = registry.stats();
    println!("  Total: {}", stats.total);
    println!("  Running: {}", stats.running);
    println!("  Stopped: {}", stats.stopped);

    // Demonstrate API version compatibility
    println!("\n=== API Version Compatibility ===");

    let host_v = ApiVersion::new(0, 18, 5);
    let plugin_v1 = ApiVersion::new(0, 18, 0);
    let plugin_v2 = ApiVersion::new(0, 19, 0);
    let plugin_v3 = ApiVersion::new(1, 0, 0);

    println!("Host version: {}", host_v);
    println!("Plugin 0.18.0 compatible: {}", host_v.is_compatible_with(&plugin_v1));
    println!("Plugin 0.19.0 compatible: {}", host_v.is_compatible_with(&plugin_v2));
    println!("Plugin 1.0.0 compatible: {}", host_v.is_compatible_with(&plugin_v3));

    // Show manifest serialization
    println!("\n=== Manifest Serialization ===");

    let toml_str = manifest.to_toml()?;
    println!("TOML output:\n{}", toml_str);

    println!("\n=== Example Complete ===");

    Ok(())
}
