# Multi-Process Safety (vNEXT)

This guide covers multi-process plugin execution with sandboxing and isolation.

## Table of Contents

- [Overview](#overview)
- [Sandboxing Architecture](#sandboxing-architecture)
- [Process Isolation](#process-isolation)
- [Inter-Process Communication](#inter-process-communication)
- [Concurrent Access Patterns](#concurrent-access-patterns)
- [Safety Guarantees](#safety-guarantees)
- [Examples](#examples)

## Overview

Multi-process safety enables:
- **Process isolation**: Plugins run in separate processes
- **Resource limits**: Per-plugin memory and CPU constraints
- **Crash isolation**: Plugin crashes don't affect host
- **Concurrent execution**: Multiple plugins run simultaneously
- **IPC security**: Controlled communication between processes

### Architecture

```
Host Process
    ↓
Plugin Runtime (Coordinator)
    ↓
├── Plugin Process 1 (sandboxed)
├── Plugin Process 2 (sandboxed)
└── Plugin Process 3 (sandboxed)
```

## Sandboxing Architecture

### Process-Based Sandboxing

Each plugin runs in its own OS process with resource limits:

```rust
use fusabi_plugin_runtime::{PluginLoader, LoaderConfig, SandboxConfig};
use std::time::Duration;

let sandbox = SandboxConfig::new()
    .with_max_memory(50 * 1024 * 1024)  // 50MB memory limit
    .with_max_cpu_percent(25.0)          // 25% CPU limit
    .with_timeout(Duration::from_secs(30))  // 30s execution timeout
    .with_process_isolation(true);       // Run in separate process

let config = LoaderConfig::new()
    .with_sandbox_config(sandbox);

let loader = PluginLoader::new(config)?;
```

### Sandbox Features

1. **Memory Limits**: Prevent memory exhaustion
2. **CPU Limits**: Prevent CPU hogging
3. **Timeout Enforcement**: Kill runaway plugins
4. **File Descriptor Limits**: Prevent fd exhaustion
5. **Network Isolation**: Optional network namespace

### Implementation Strategy

**Current (In-Process)**:
```rust
// Plugins run in same process as host
// Fast but less isolated
let plugin = loader.load_source("plugin.fsx")?;
let result = plugin.call("main", &[])?;
```

**Future (Multi-Process)**:
```rust
// Plugins run in separate processes
// Slower but fully isolated
let plugin = loader.load_source_sandboxed("plugin.fsx")?;
let result = plugin.call("main", &[])?; // IPC under the hood
```

## Process Isolation

### Linux Namespaces (Future Enhancement)

Use Linux namespaces for strong isolation:

```rust
pub struct NamespaceConfig {
    pub pid: bool,      // PID namespace (separate process tree)
    pub net: bool,      // Network namespace (isolated network)
    pub mnt: bool,      // Mount namespace (isolated filesystem view)
    pub ipc: bool,      // IPC namespace (isolated shared memory)
    pub uts: bool,      // UTS namespace (isolated hostname)
}

let config = SandboxConfig::new()
    .with_namespaces(NamespaceConfig {
        pid: true,
        net: true,
        mnt: false,  // Share filesystem
        ipc: true,
        uts: false,
    });
```

### Resource Control (cgroups)

Use cgroups for resource limits:

```rust
pub struct ResourceLimits {
    pub memory_bytes: u64,
    pub cpu_shares: u64,
    pub io_weight: u16,
}

let limits = ResourceLimits {
    memory_bytes: 100 * 1024 * 1024,  // 100MB
    cpu_shares: 512,                   // 50% of CPU (out of 1024)
    io_weight: 100,                    // Default I/O priority
};

let config = SandboxConfig::new()
    .with_resource_limits(limits);
```

### Capability Dropping

Drop unnecessary Linux capabilities:

```rust
let config = SandboxConfig::new()
    .drop_capabilities(&[
        "CAP_SYS_ADMIN",
        "CAP_NET_ADMIN",
        "CAP_SYS_MODULE",
    ])
    .keep_capabilities(&[
        "CAP_NET_BIND_SERVICE",  // If plugin needs to bind <1024
    ]);
```

## Inter-Process Communication

### Message Passing

Plugins communicate via message passing:

```rust
use fusabi_plugin_runtime::{IpcMessage, IpcChannel};

// In host process
let (tx, rx) = IpcChannel::new()?;

// Send message to plugin
tx.send(IpcMessage::Call {
    function: "process".to_string(),
    args: vec![Value::String("data".into())],
})?;

// Receive response
match rx.recv()? {
    IpcMessage::Result(value) => {
        println!("Plugin returned: {:?}", value);
    }
    IpcMessage::Error(err) => {
        eprintln!("Plugin error: {}", err);
    }
    _ => {}
}
```

### Shared Memory (Future Enhancement)

For high-performance scenarios:

```rust
use fusabi_plugin_runtime::SharedMemory;

// Create shared memory region
let shm = SharedMemory::new(1024 * 1024)?; // 1MB

// Write data
shm.write(0, &data)?;

// Plugin reads from shared memory
// Requires explicit capability grant
```

### RPC Protocol

```rust
// Plugin -> Host: Function call
{
    "type": "call",
    "function": "process",
    "args": [{"String": "data"}]
}

// Host -> Plugin: Result
{
    "type": "result",
    "value": {"String": "processed"}
}

// Host -> Plugin: Error
{
    "type": "error",
    "message": "Processing failed"
}
```

## Concurrent Access Patterns

### Thread-Safe Registry

The registry supports concurrent access from multiple threads:

```rust
use fusabi_plugin_runtime::PluginRegistry;
use std::sync::Arc;
use std::thread;

let registry = Arc::new(PluginRegistry::new(RegistryConfig::default()));

// Multiple threads can access registry simultaneously
let handles: Vec<_> = (0..10)
    .map(|i| {
        let registry = Arc::clone(&registry);
        thread::spawn(move || {
            // Each thread can load/access plugins
            if let Ok(plugin) = registry.get("worker") {
                plugin.call("process", &[Value::Number(i as f64)])
            } else {
                Ok(Value::Null)
            }
        })
    })
    .collect();

for handle in handles {
    handle.join().unwrap()?;
}
```

### Plugin Pool Pattern

Execute plugins concurrently with a pool:

```rust
use fusabi_plugin_runtime::{PluginPool, PoolConfig};
use tokio::sync::mpsc;

let config = PoolConfig::new()
    .with_size(4)  // 4 worker processes
    .with_queue_size(100);

let pool = PluginPool::new("worker-plugin", config)?;

// Submit tasks to pool
let (tx, mut rx) = mpsc::channel(100);

for i in 0..100 {
    let tx = tx.clone();
    pool.execute(move |plugin| {
        let result = plugin.call("work", &[Value::Number(i as f64)])?;
        tx.send(result).await.ok();
        Ok(())
    }).await?;
}

drop(tx);

// Collect results
while let Some(result) = rx.recv().await {
    println!("Task result: {:?}", result);
}
```

### Lock-Free Access

For read-heavy workloads:

```rust
use parking_lot::RwLock;
use std::sync::Arc;

struct PluginCache {
    plugins: Arc<RwLock<HashMap<String, Arc<Plugin>>>>,
}

impl PluginCache {
    // Multiple readers can access concurrently
    pub fn get(&self, name: &str) -> Option<Arc<Plugin>> {
        self.plugins.read().get(name).cloned()
    }

    // Writer has exclusive access
    pub fn insert(&self, name: String, plugin: Plugin) {
        self.plugins.write().insert(name, Arc::new(plugin));
    }
}
```

## Safety Guarantees

### Memory Safety

1. **Isolated heaps**: Each plugin has separate memory
2. **Bounds checking**: All memory access validated
3. **No shared state**: Plugins can't access host memory
4. **Automatic cleanup**: Memory freed on plugin exit

### Crash Isolation

```rust
use fusabi_plugin_runtime::{PluginHandle, PluginStatus};

let handle = runtime.spawn_plugin("risky-plugin")?;

// Monitor plugin health
loop {
    match handle.status() {
        PluginStatus::Running => {
            // Plugin is healthy
        }
        PluginStatus::Crashed { exit_code, signal } => {
            eprintln!("Plugin crashed: exit={}, signal={:?}", exit_code, signal);

            // Restart plugin
            runtime.restart_plugin("risky-plugin")?;
            break;
        }
        PluginStatus::Terminated => {
            println!("Plugin terminated normally");
            break;
        }
    }

    std::thread::sleep(Duration::from_secs(1));
}
```

### Deadlock Prevention

1. **Timeout enforcement**: All operations have timeouts
2. **Non-blocking IPC**: Async message passing
3. **Lock ordering**: Consistent lock acquisition order
4. **Lock-free structures**: Use DashMap, Arc, etc.

### Data Race Prevention

1. **Message passing**: Prefer messages over shared memory
2. **Immutable data**: Pass immutable values when possible
3. **Synchronization primitives**: Use Mutex, RwLock correctly
4. **Type safety**: Rust's type system prevents data races

## Examples

### Example 1: Sandboxed Plugin Execution

```rust
use fusabi_plugin_runtime::{
    PluginLoader, LoaderConfig, SandboxConfig
};
use std::time::Duration;

fn run_sandboxed() -> Result<()> {
    let sandbox = SandboxConfig::new()
        .with_max_memory(50 * 1024 * 1024)
        .with_timeout(Duration::from_secs(10))
        .with_process_isolation(true);

    let config = LoaderConfig::new()
        .with_sandbox_config(sandbox);

    let loader = PluginLoader::new(config)?;
    let plugin = loader.load_source("untrusted.fsx")?;

    // Plugin runs in separate process with limits
    match plugin.call("main", &[]) {
        Ok(result) => println!("Success: {:?}", result),
        Err(e) => eprintln!("Plugin error: {}", e),
    }

    Ok(())
}
```

### Example 2: Concurrent Plugin Execution

```rust
use tokio::task;
use futures::future::join_all;

async fn run_concurrent() -> Result<()> {
    let runtime = PluginRuntime::new(RuntimeConfig::default())?;

    runtime.load_manifest("plugin-a.toml")?;
    runtime.load_manifest("plugin-b.toml")?;
    runtime.load_manifest("plugin-c.toml")?;

    // Execute plugins concurrently
    let tasks = vec![
        task::spawn(async move {
            runtime.call("plugin-a", "work", &[]).await
        }),
        task::spawn(async move {
            runtime.call("plugin-b", "work", &[]).await
        }),
        task::spawn(async move {
            runtime.call("plugin-c", "work", &[]).await
        }),
    ];

    let results = join_all(tasks).await;

    for result in results {
        match result {
            Ok(Ok(value)) => println!("Result: {:?}", value),
            Ok(Err(e)) => eprintln!("Plugin error: {}", e),
            Err(e) => eprintln!("Task error: {}", e),
        }
    }

    Ok(())
}
```

### Example 3: Plugin Pool

```rust
use fusabi_plugin_runtime::{PluginPool, PoolConfig};

fn process_with_pool() -> Result<()> {
    let config = PoolConfig::new()
        .with_size(4)
        .with_restart_on_crash(true);

    let pool = PluginPool::new("worker", config)?;

    // Process 1000 items with 4 worker processes
    let items: Vec<_> = (0..1000).collect();

    for item in items {
        pool.submit(move |plugin| {
            plugin.call("process", &[Value::Number(item as f64)])
        })?;
    }

    // Wait for all tasks to complete
    pool.wait()?;

    println!("Processed {} items", 1000);

    Ok(())
}
```

### Example 4: Crash Recovery

```rust
use fusabi_plugin_runtime::{PluginRuntime, PluginStatus};
use std::time::Duration;

fn resilient_execution() -> Result<()> {
    let runtime = PluginRuntime::new(RuntimeConfig::default())?;
    runtime.load_manifest("flaky-plugin.toml")?;

    let max_retries = 3;
    let mut retries = 0;

    loop {
        match runtime.call("flaky-plugin", "work", &[]) {
            Ok(result) => {
                println!("Success: {:?}", result);
                break;
            }
            Err(e) if retries < max_retries => {
                eprintln!("Attempt {} failed: {}", retries + 1, e);
                retries += 1;

                // Restart plugin
                runtime.restart_plugin("flaky-plugin")?;

                // Exponential backoff
                let delay = Duration::from_secs(2u64.pow(retries));
                std::thread::sleep(delay);
            }
            Err(e) => {
                eprintln!("Failed after {} retries: {}", max_retries, e);
                return Err(e);
            }
        }
    }

    Ok(())
}
```

## Best Practices

1. **Use process isolation for untrusted plugins**
2. **Set appropriate resource limits**
3. **Implement timeout enforcement**
4. **Monitor plugin health**
5. **Handle crashes gracefully**
6. **Use async/await for concurrent execution**
7. **Prefer message passing over shared memory**
8. **Log all IPC for debugging**

## Performance Considerations

### Process Creation Overhead

- Process spawning is slower than in-process
- Consider plugin pools for repeated execution
- Reuse processes when possible

### IPC Overhead

- Message serialization has cost
- Batch messages when possible
- Use shared memory for large data transfers

### Context Switching

- Minimize cross-process calls
- Keep plugin logic self-contained
- Cache results when appropriate

## See Also

- [Runtime Guide](runtime-guide.md)
- [Capabilities Guide](capabilities.md)
- [Host Profiles](host-profiles.md)
- [Hot Reload](hot-reload.md)
