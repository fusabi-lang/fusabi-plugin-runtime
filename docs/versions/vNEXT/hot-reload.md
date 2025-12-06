# Hot Reload (vNEXT)

This guide covers hot reload functionality with debouncing and backoff strategies.

## Table of Contents

- [Overview](#overview)
- [Watcher Configuration](#watcher-configuration)
- [Debounce Strategy](#debounce-strategy)
- [Backoff Strategy](#backoff-strategy)
- [Event Handling](#event-handling)
- [State Preservation](#state-preservation)
- [Error Recovery](#error-recovery)
- [Examples](#examples)

## Overview

Hot reload enables automatic plugin reloading when source files change, ideal for:
- Development workflows
- Live configuration updates
- Automatic plugin updates
- Dynamic plugin management

### How It Works

```
File Change Detected
       ↓
Debounce Timer (prevent reload storms)
       ↓
Reload Attempt
       ↓
Success → Plugin Updated
       ↓
Failure → Exponential Backoff → Retry
```

## Watcher Configuration

### Basic Watcher Setup

```rust
use fusabi_plugin_runtime::{PluginWatcher, WatchConfig};
use std::time::Duration;

let config = WatchConfig::new()
    .with_debounce(Duration::from_millis(500))  // Wait 500ms after last change
    .with_recursive(true)                        // Watch subdirectories
    .with_follow_symlinks(false);                // Don't follow symlinks

let watcher = PluginWatcher::new(config)?;
```

### Watch Specific Paths

```rust
// Watch a directory
watcher.watch("plugins/")?;

// Watch a specific file
watcher.watch("plugins/my-plugin/plugin.toml")?;

// Watch multiple paths
watcher.watch_all(&[
    "plugins/plugin-a/",
    "plugins/plugin-b/",
    "config/plugins.toml",
])?;
```

### File Filtering

```rust
let config = WatchConfig::new()
    .with_extensions(&["fsx", "toml"])  // Only watch these extensions
    .with_exclude_patterns(&[
        "*.tmp",
        "*.swp",
        ".git/*",
        "target/*",
    ]);

let watcher = PluginWatcher::new(config)?;
```

## Debounce Strategy

Debouncing prevents reload storms when multiple files change rapidly:

### Fixed Debounce

Simple fixed delay after last change:

```rust
let config = WatchConfig::new()
    .with_debounce(Duration::from_millis(500));

// Timeline:
// t=0ms:   File change #1 → Start timer
// t=100ms: File change #2 → Restart timer
// t=200ms: File change #3 → Restart timer
// t=700ms: Timer expires → Reload
```

### Adaptive Debounce

Adjust debounce based on change frequency:

```rust
let config = WatchConfig::new()
    .with_adaptive_debounce(
        Duration::from_millis(200),  // Min debounce
        Duration::from_secs(5),       // Max debounce
    );

// More frequent changes → longer debounce
// Fewer changes → shorter debounce
```

### Custom Debounce Logic

```rust
use fusabi_plugin_runtime::{WatchEvent, DebouncerConfig};

let debouncer = DebouncerConfig::custom(|events: &[WatchEvent]| {
    // Custom logic based on events
    if events.len() > 10 {
        Duration::from_secs(2)  // Many changes → longer debounce
    } else {
        Duration::from_millis(500)  // Few changes → shorter debounce
    }
});

let config = WatchConfig::new()
    .with_debouncer(debouncer);
```

## Backoff Strategy

Exponential backoff for failed reload attempts:

### Exponential Backoff

```rust
let config = WatchConfig::new()
    .with_backoff(BackoffConfig::exponential()
        .with_initial_delay(Duration::from_millis(100))
        .with_max_delay(Duration::from_secs(30))
        .with_multiplier(2.0)
        .with_max_retries(5)
    );

// Retry delays:
// Attempt 1: 100ms
// Attempt 2: 200ms
// Attempt 3: 400ms
// Attempt 4: 800ms
// Attempt 5: 1600ms
// Attempt 6: 3200ms (capped at max_retries)
```

### Linear Backoff

```rust
let config = WatchConfig::new()
    .with_backoff(BackoffConfig::linear()
        .with_initial_delay(Duration::from_secs(1))
        .with_increment(Duration::from_secs(1))
        .with_max_retries(3)
    );

// Retry delays:
// Attempt 1: 1s
// Attempt 2: 2s
// Attempt 3: 3s
```

### Custom Backoff

```rust
let config = WatchConfig::new()
    .with_backoff(BackoffConfig::custom(|attempt| {
        // Fibonacci backoff
        let fib = fibonacci(attempt);
        Duration::from_secs(fib)
    }));
```

## Event Handling

### Watch Events

```rust
use fusabi_plugin_runtime::WatchEvent;

watcher.on_event(|event| {
    match event {
        WatchEvent::Created { path } => {
            println!("File created: {}", path.display());
        }
        WatchEvent::Modified { path } => {
            println!("File modified: {}", path.display());
        }
        WatchEvent::Deleted { path } => {
            println!("File deleted: {}", path.display());
        }
        WatchEvent::Renamed { from, to } => {
            println!("File renamed: {} -> {}", from.display(), to.display());
        }
    }
});
```

### Reload Handler

```rust
use fusabi_plugin_runtime::{PluginRuntime, RuntimeConfig};

let runtime = PluginRuntime::new(RuntimeConfig::default())?;
let mut watcher = PluginWatcher::new(WatchConfig::default())?;

watcher.on_change(move |event| {
    if let WatchEvent::Modified { path } = event {
        // Extract plugin name from path
        if let Some(plugin_name) = extract_plugin_name(&path) {
            println!("Reloading plugin: {}", plugin_name);

            match runtime.reload_plugin(&plugin_name) {
                Ok(_) => println!("Successfully reloaded {}", plugin_name),
                Err(e) => eprintln!("Failed to reload {}: {}", plugin_name, e),
            }
        }
    }
});

watcher.watch("plugins/")?;
watcher.start()?;
```

### Lifecycle Hooks

```rust
watcher
    .on_before_reload(|plugin_name| {
        println!("Preparing to reload: {}", plugin_name);
        // Save state, cleanup resources, etc.
    })
    .on_after_reload(|plugin_name, result| {
        match result {
            Ok(_) => println!("Reload complete: {}", plugin_name),
            Err(e) => eprintln!("Reload failed: {}: {}", plugin_name, e),
        }
    })
    .on_retry(|plugin_name, attempt| {
        println!("Retry attempt {} for {}", attempt, plugin_name);
    });
```

## State Preservation

### Automatic State Saving

```rust
use fusabi_plugin_runtime::{PluginState, StateManager};

let state_manager = StateManager::new("~/.fusabi/state");

watcher.on_before_reload(move |plugin_name| {
    // Save plugin state before reload
    if let Ok(state) = runtime.get_plugin_state(plugin_name) {
        state_manager.save(plugin_name, &state)?;
    }
});

watcher.on_after_reload(move |plugin_name, result| {
    if result.is_ok() {
        // Restore state after successful reload
        if let Ok(state) = state_manager.load(plugin_name) {
            runtime.restore_plugin_state(plugin_name, state)?;
        }
    }
});
```

### Manual State Management

```rust
// In plugin code
fn save_state() -> Result<Value> {
    let state = json!({
        "counter": counter,
        "data": data,
        "timestamp": current_time(),
    });
    Ok(Value::from(state))
}

fn restore_state(state: Value) -> Result<()> {
    counter = state["counter"].as_i64()?;
    data = state["data"].clone();
    Ok(())
}
```

## Error Recovery

### Reload Error Handling

```rust
watcher.on_change(move |event| {
    if let WatchEvent::Modified { path } = event {
        if let Some(plugin_name) = extract_plugin_name(&path) {
            match runtime.reload_plugin(&plugin_name) {
                Ok(_) => {
                    println!("✓ Reloaded {}", plugin_name);
                }
                Err(Error::CompilationFailed(msg)) => {
                    eprintln!("✗ Compilation error in {}: {}", plugin_name, msg);
                    // Keep old version running
                }
                Err(Error::ManifestParse(msg)) => {
                    eprintln!("✗ Invalid manifest in {}: {}", plugin_name, msg);
                    // Keep old version running
                }
                Err(e) => {
                    eprintln!("✗ Reload failed for {}: {}", plugin_name, e);
                    // Attempt recovery
                    runtime.restart_plugin(&plugin_name)?;
                }
            }
        }
    }
});
```

### Fallback to Previous Version

```rust
use std::collections::HashMap;

struct VersionedPlugin {
    current: Plugin,
    previous: Option<Plugin>,
}

let mut plugins: HashMap<String, VersionedPlugin> = HashMap::new();

watcher.on_change(move |event| {
    if let Some(plugin_name) = extract_plugin_name(&event.path()) {
        let versioned = plugins.get_mut(&plugin_name).unwrap();

        // Try to reload
        match runtime.reload_plugin(&plugin_name) {
            Ok(new_plugin) => {
                // Success - update versions
                versioned.previous = Some(versioned.current.clone());
                versioned.current = new_plugin;
            }
            Err(e) => {
                eprintln!("Reload failed: {}", e);

                // Rollback to previous version
                if let Some(prev) = &versioned.previous {
                    runtime.replace_plugin(&plugin_name, prev.clone())?;
                    println!("Rolled back to previous version");
                }
            }
        }
    }
});
```

### Circuit Breaker Pattern

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct CircuitBreaker {
    failures: AtomicUsize,
    last_failure: Mutex<Option<Instant>>,
    threshold: usize,
    timeout: Duration,
}

impl CircuitBreaker {
    fn should_attempt(&self) -> bool {
        let failures = self.failures.load(Ordering::Relaxed);

        if failures < self.threshold {
            return true;
        }

        // Check if timeout expired
        if let Some(last) = *self.last_failure.lock().unwrap() {
            if last.elapsed() > self.timeout {
                // Reset circuit breaker
                self.failures.store(0, Ordering::Relaxed);
                return true;
            }
        }

        false
    }

    fn on_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
    }

    fn on_failure(&self) {
        self.failures.fetch_add(1, Ordering::Relaxed);
        *self.last_failure.lock().unwrap() = Some(Instant::now());
    }
}

let breaker = CircuitBreaker {
    failures: AtomicUsize::new(0),
    last_failure: Mutex::new(None),
    threshold: 3,
    timeout: Duration::from_secs(60),
};

watcher.on_change(move |event| {
    if breaker.should_attempt() {
        match runtime.reload_plugin(&plugin_name) {
            Ok(_) => breaker.on_success(),
            Err(_) => breaker.on_failure(),
        }
    } else {
        println!("Circuit breaker open, skipping reload");
    }
});
```

## Examples

### Example 1: Development Workflow

```rust
use fusabi_plugin_runtime::{PluginRuntime, RuntimeConfig, PluginWatcher, WatchConfig};
use std::time::Duration;

fn development_mode() -> Result<()> {
    let runtime = PluginRuntime::new(RuntimeConfig::default())?;

    // Load initial plugin
    runtime.load_manifest("plugins/dev-plugin/plugin.toml")?;

    // Setup hot reload with short debounce for development
    let config = WatchConfig::new()
        .with_debounce(Duration::from_millis(200))
        .with_extensions(&["fsx", "toml"]);

    let mut watcher = PluginWatcher::new(config)?;

    watcher.on_change(move |event| {
        println!("\n🔄 Change detected: {:?}", event);

        if let Some(plugin_name) = extract_plugin_name(&event.path()) {
            match runtime.reload_plugin(&plugin_name) {
                Ok(_) => println!("✓ Reloaded successfully"),
                Err(e) => eprintln!("✗ Reload failed: {}", e),
            }
        }
    });

    watcher.watch("plugins/")?;
    watcher.start()?;

    println!("🚀 Development mode active. Watching for changes...");

    // Keep running
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
```

### Example 2: Production with Backoff

```rust
fn production_mode() -> Result<()> {
    let runtime = PluginRuntime::new(RuntimeConfig::default())?;

    // Conservative configuration for production
    let config = WatchConfig::new()
        .with_debounce(Duration::from_secs(2))  // Longer debounce
        .with_backoff(BackoffConfig::exponential()
            .with_initial_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(60))
            .with_max_retries(5)
        );

    let mut watcher = PluginWatcher::new(config)?;

    watcher.on_change(move |event| {
        tracing::info!("Plugin change detected: {:?}", event);

        if let Some(plugin_name) = extract_plugin_name(&event.path()) {
            match runtime.reload_plugin(&plugin_name) {
                Ok(_) => {
                    tracing::info!("Plugin reloaded: {}", plugin_name);
                }
                Err(e) => {
                    tracing::error!("Reload failed: {}: {}", plugin_name, e);
                    // Alert monitoring system
                }
            }
        }
    });

    watcher.watch("/etc/fusabi/plugins/")?;
    watcher.start()?;

    Ok(())
}
```

### Example 3: Selective Reloading

```rust
fn selective_reload() -> Result<()> {
    let runtime = PluginRuntime::new(RuntimeConfig::default())?;
    let mut watcher = PluginWatcher::new(WatchConfig::default())?;

    watcher.on_change(move |event| {
        let path = event.path();

        // Only reload if specific files changed
        if path.ends_with("plugin.toml") {
            // Manifest changed - full reload
            println!("Manifest changed, full reload");
            if let Some(plugin_name) = extract_plugin_name(&path) {
                runtime.reload_plugin(&plugin_name)?;
            }
        } else if path.extension() == Some("fsx") {
            // Source changed - check if compilation succeeds before reload
            println!("Source changed, validating before reload");

            if let Some(plugin_name) = extract_plugin_name(&path) {
                if runtime.validate_plugin_source(&plugin_name).is_ok() {
                    runtime.reload_plugin(&plugin_name)?;
                } else {
                    eprintln!("Validation failed, keeping current version");
                }
            }
        }
    });

    watcher.watch("plugins/")?;
    watcher.start()?;

    Ok(())
}
```

## Best Practices

1. **Use appropriate debounce**: Short for development, longer for production
2. **Implement backoff**: Prevent rapid retry loops
3. **Preserve state**: Save/restore plugin state across reloads
4. **Validate before reload**: Check compilation before switching
5. **Monitor reload health**: Track success/failure rates
6. **Use circuit breaker**: Prevent reload storms
7. **Log reload events**: Audit trail for debugging

## Performance Considerations

- **Debounce overhead**: Balance responsiveness vs. reload frequency
- **State serialization**: Minimize state size for faster reloads
- **Compilation time**: Pre-compile to bytecode for faster reloads
- **File watching**: Limit watched paths to reduce overhead

## See Also

- [Runtime Guide](runtime-guide.md)
- [Multi-Process Safety](multi-process.md)
- [Migration Guide](migration.md)
