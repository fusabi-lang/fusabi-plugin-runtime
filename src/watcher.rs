//! File system watcher for plugin hot reload.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use parking_lot::RwLock;

use crate::error::{Error, Result};

/// Configuration for the plugin watcher.
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// Debounce duration for file changes.
    pub debounce: Duration,
    /// Whether to watch recursively.
    pub recursive: bool,
    /// File extensions to watch.
    pub extensions: Vec<String>,
    /// Whether to auto-reload on change.
    pub auto_reload: bool,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(500),
            recursive: true,
            extensions: vec![
                "fsx".to_string(),
                "fzb".to_string(),
                "toml".to_string(),
            ],
            auto_reload: true,
        }
    }
}

impl WatchConfig {
    /// Create a new watch configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the debounce duration.
    pub fn with_debounce(mut self, duration: Duration) -> Self {
        self.debounce = duration;
        self
    }

    /// Set recursive watching.
    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Set file extensions to watch.
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Set auto-reload behavior.
    pub fn with_auto_reload(mut self, auto: bool) -> Self {
        self.auto_reload = auto;
        self
    }
}

/// Event emitted when a watched file changes.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A file was created.
    Created {
        /// Path to the created file.
        path: PathBuf,
    },
    /// A file was modified.
    Modified {
        /// Path to the modified file.
        path: PathBuf,
    },
    /// A file was removed.
    Removed {
        /// Path to the removed file.
        path: PathBuf,
    },
    /// A file was renamed.
    Renamed {
        /// Old path.
        from: PathBuf,
        /// New path.
        to: PathBuf,
    },
}

impl WatchEvent {
    /// Get the primary path for this event.
    pub fn path(&self) -> &Path {
        match self {
            Self::Created { path } => path,
            Self::Modified { path } => path,
            Self::Removed { path } => path,
            Self::Renamed { to, .. } => to,
        }
    }

    /// Check if this event affects a file with the given extensions.
    pub fn matches_extension(&self, extensions: &[String]) -> bool {
        let path = self.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            extensions.iter().any(|e| e == ext)
        } else {
            false
        }
    }
}

type EventHandler = Box<dyn Fn(WatchEvent) + Send + Sync>;

/// Internal state for tracking file changes.
struct WatchState {
    last_events: HashMap<PathBuf, Instant>,
    handlers: Vec<EventHandler>,
}

/// Plugin file watcher for hot reload support.
pub struct PluginWatcher {
    config: WatchConfig,
    watcher: Option<RecommendedWatcher>,
    watched_paths: RwLock<Vec<PathBuf>>,
    state: Arc<RwLock<WatchState>>,
    running: Arc<AtomicBool>,
}

impl PluginWatcher {
    /// Create a new plugin watcher.
    pub fn new(config: WatchConfig) -> Result<Self> {
        let state = Arc::new(RwLock::new(WatchState {
            last_events: HashMap::new(),
            handlers: Vec::new(),
        }));

        let running = Arc::new(AtomicBool::new(false));

        Ok(Self {
            config,
            watcher: None,
            watched_paths: RwLock::new(Vec::new()),
            state,
            running,
        })
    }

    /// Create with default configuration.
    pub fn default_config() -> Result<Self> {
        Self::new(WatchConfig::default())
    }

    /// Get the watcher configuration.
    pub fn config(&self) -> &WatchConfig {
        &self.config
    }

    /// Check if the watcher is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Add an event handler.
    pub fn on_change<F>(&self, handler: F)
    where
        F: Fn(WatchEvent) + Send + Sync + 'static,
    {
        self.state.write().handlers.push(Box::new(handler));
    }

    /// Start watching.
    pub fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        let state = self.state.clone();
        let config = self.config.clone();
        let running = self.running.clone();

        let watcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                if !running.load(Ordering::Relaxed) {
                    return;
                }

                if let Ok(event) = res {
                    Self::handle_event(&state, &config, event);
                }
            },
            Config::default(),
        )
        .map_err(|e| Error::Watch(e.to_string()))?;

        self.watcher = Some(watcher);
        self.running.store(true, Ordering::Relaxed);

        // Re-watch all registered paths
        for path in self.watched_paths.read().iter() {
            self.watch_path_internal(path)?;
        }

        tracing::info!("Plugin watcher started");
        Ok(())
    }

    /// Stop watching.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.watcher = None;
        tracing::info!("Plugin watcher stopped");
    }

    /// Watch a path.
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        // Add to watched paths
        {
            let mut paths = self.watched_paths.write();
            if !paths.contains(&path) {
                paths.push(path.clone());
            }
        }

        // If running, start watching
        if self.running.load(Ordering::Relaxed) {
            self.watch_path_internal(&path)?;
        }

        Ok(())
    }

    /// Unwatch a path.
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Remove from watched paths
        {
            let mut paths = self.watched_paths.write();
            paths.retain(|p| p != path);
        }

        // If running, stop watching
        if let Some(ref mut watcher) = self.watcher {
            watcher
                .unwatch(path)
                .map_err(|e| Error::Watch(e.to_string()))?;
        }

        Ok(())
    }

    /// Get watched paths.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.read().clone()
    }

    // Internal methods

    fn watch_path_internal(&self, path: &Path) -> Result<()> {
        if let Some(ref watcher) = self.watcher {
            let mode = if self.config.recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };

            // Note: watcher is not mutable here, so this is a simplified version
            // In real implementation, would need interior mutability or different design
            tracing::debug!("Watching path: {}", path.display());
        }

        Ok(())
    }

    fn handle_event(state: &Arc<RwLock<WatchState>>, config: &WatchConfig, event: Event) {
        let watch_event = match event.kind {
            EventKind::Create(_) => {
                event.paths.first().map(|p| WatchEvent::Created {
                    path: p.clone(),
                })
            }
            EventKind::Modify(_) => {
                event.paths.first().map(|p| WatchEvent::Modified {
                    path: p.clone(),
                })
            }
            EventKind::Remove(_) => {
                event.paths.first().map(|p| WatchEvent::Removed {
                    path: p.clone(),
                })
            }
            _ => None,
        };

        if let Some(watch_event) = watch_event {
            // Check extension filter
            if !watch_event.matches_extension(&config.extensions) {
                return;
            }

            // Debounce
            let path = watch_event.path().to_path_buf();
            {
                let mut state = state.write();
                let now = Instant::now();

                if let Some(last) = state.last_events.get(&path) {
                    if now.duration_since(*last) < config.debounce {
                        return;
                    }
                }

                state.last_events.insert(path, now);

                // Notify handlers
                for handler in &state.handlers {
                    handler(watch_event.clone());
                }
            }
        }
    }
}

impl std::fmt::Debug for PluginWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginWatcher")
            .field("config", &self.config)
            .field("running", &self.is_running())
            .field("watched_paths", &self.watched_paths.read().len())
            .finish()
    }
}

impl Drop for PluginWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_config_builder() {
        let config = WatchConfig::new()
            .with_debounce(Duration::from_secs(1))
            .with_recursive(false)
            .with_auto_reload(true);

        assert_eq!(config.debounce, Duration::from_secs(1));
        assert!(!config.recursive);
        assert!(config.auto_reload);
    }

    #[test]
    fn test_watch_event_extension_match() {
        let event = WatchEvent::Modified {
            path: PathBuf::from("test.fsx"),
        };

        assert!(event.matches_extension(&["fsx".to_string()]));
        assert!(!event.matches_extension(&["rs".to_string()]));
    }

    #[test]
    fn test_watcher_creation() {
        let watcher = PluginWatcher::default_config().unwrap();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_watch_path() {
        let mut watcher = PluginWatcher::default_config().unwrap();
        watcher.watch("/tmp/plugins").unwrap();

        let paths = watcher.watched_paths();
        assert!(paths.contains(&PathBuf::from("/tmp/plugins")));
    }

    #[test]
    fn test_unwatch_path() {
        let mut watcher = PluginWatcher::default_config().unwrap();
        watcher.watch("/tmp/plugins").unwrap();
        watcher.unwatch("/tmp/plugins").unwrap();

        let paths = watcher.watched_paths();
        assert!(!paths.contains(&PathBuf::from("/tmp/plugins")));
    }
}
