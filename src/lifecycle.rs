//! Plugin lifecycle management.

use std::time::Instant;

/// Plugin lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleState {
    /// Plugin has been created but not initialized.
    Created,
    /// Plugin has been initialized with an engine.
    Initialized,
    /// Plugin is running and accepting calls.
    Running,
    /// Plugin has been stopped.
    Stopped,
    /// Plugin has been unloaded.
    Unloaded,
    /// Plugin is in an error state.
    Error,
}

impl LifecycleState {
    /// Check if the plugin can be started.
    pub fn can_start(&self) -> bool {
        matches!(self, Self::Initialized)
    }

    /// Check if the plugin can be stopped.
    pub fn can_stop(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Check if the plugin can be called.
    pub fn can_call(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Check if the plugin can be reloaded.
    pub fn can_reload(&self) -> bool {
        matches!(self, Self::Initialized | Self::Running | Self::Stopped | Self::Error)
    }

    /// Check if the plugin is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Unloaded)
    }

    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Created => "Plugin created but not initialized",
            Self::Initialized => "Plugin initialized and ready to start",
            Self::Running => "Plugin running and accepting calls",
            Self::Stopped => "Plugin stopped",
            Self::Unloaded => "Plugin unloaded",
            Self::Error => "Plugin in error state",
        }
    }
}

impl std::fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Created => "created",
            Self::Initialized => "initialized",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Unloaded => "unloaded",
            Self::Error => "error",
        };
        write!(f, "{}", name)
    }
}

/// Trait for plugin lifecycle management.
pub trait PluginLifecycle {
    /// Initialize the plugin.
    fn on_init(&mut self) -> crate::Result<()> {
        Ok(())
    }

    /// Start the plugin.
    fn on_start(&mut self) -> crate::Result<()> {
        Ok(())
    }

    /// Stop the plugin.
    fn on_stop(&mut self) -> crate::Result<()> {
        Ok(())
    }

    /// Unload the plugin.
    fn on_unload(&mut self) -> crate::Result<()> {
        Ok(())
    }

    /// Called before a reload.
    fn on_before_reload(&mut self) -> crate::Result<()> {
        Ok(())
    }

    /// Called after a reload.
    fn on_after_reload(&mut self) -> crate::Result<()> {
        Ok(())
    }

    /// Called when an error occurs.
    fn on_error(&mut self, error: &crate::Error) {
        tracing::error!("Plugin error: {}", error);
    }
}

/// Lifecycle event for hooks.
#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    /// Plugin was created.
    Created {
        /// Plugin name.
        name: String,
        /// Creation time.
        at: Instant,
    },
    /// Plugin was initialized.
    Initialized {
        /// Plugin name.
        name: String,
        /// Initialization time.
        at: Instant,
    },
    /// Plugin was started.
    Started {
        /// Plugin name.
        name: String,
        /// Start time.
        at: Instant,
    },
    /// Plugin was stopped.
    Stopped {
        /// Plugin name.
        name: String,
        /// Stop time.
        at: Instant,
    },
    /// Plugin was reloaded.
    Reloaded {
        /// Plugin name.
        name: String,
        /// Reload time.
        at: Instant,
        /// Reload count.
        count: u64,
    },
    /// Plugin was unloaded.
    Unloaded {
        /// Plugin name.
        name: String,
        /// Unload time.
        at: Instant,
    },
    /// Plugin encountered an error.
    Error {
        /// Plugin name.
        name: String,
        /// Error message.
        message: String,
        /// Error time.
        at: Instant,
    },
}

impl LifecycleEvent {
    /// Get the plugin name.
    pub fn plugin_name(&self) -> &str {
        match self {
            Self::Created { name, .. } => name,
            Self::Initialized { name, .. } => name,
            Self::Started { name, .. } => name,
            Self::Stopped { name, .. } => name,
            Self::Reloaded { name, .. } => name,
            Self::Unloaded { name, .. } => name,
            Self::Error { name, .. } => name,
        }
    }

    /// Get the event timestamp.
    pub fn timestamp(&self) -> Instant {
        match self {
            Self::Created { at, .. } => *at,
            Self::Initialized { at, .. } => *at,
            Self::Started { at, .. } => *at,
            Self::Stopped { at, .. } => *at,
            Self::Reloaded { at, .. } => *at,
            Self::Unloaded { at, .. } => *at,
            Self::Error { at, .. } => *at,
        }
    }

    /// Get the event name.
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::Created { .. } => "created",
            Self::Initialized { .. } => "initialized",
            Self::Started { .. } => "started",
            Self::Stopped { .. } => "stopped",
            Self::Reloaded { .. } => "reloaded",
            Self::Unloaded { .. } => "unloaded",
            Self::Error { .. } => "error",
        }
    }
}

/// Hooks for lifecycle events.
pub struct LifecycleHooks {
    handlers: Vec<Box<dyn Fn(&LifecycleEvent) + Send + Sync>>,
}

impl LifecycleHooks {
    /// Create new lifecycle hooks.
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Add a lifecycle event handler.
    pub fn on_event<F>(&mut self, handler: F)
    where
        F: Fn(&LifecycleEvent) + Send + Sync + 'static,
    {
        self.handlers.push(Box::new(handler));
    }

    /// Emit a lifecycle event.
    pub fn emit(&self, event: LifecycleEvent) {
        for handler in &self.handlers {
            handler(&event);
        }
    }

    /// Emit a created event.
    pub fn emit_created(&self, name: &str) {
        self.emit(LifecycleEvent::Created {
            name: name.to_string(),
            at: Instant::now(),
        });
    }

    /// Emit an initialized event.
    pub fn emit_initialized(&self, name: &str) {
        self.emit(LifecycleEvent::Initialized {
            name: name.to_string(),
            at: Instant::now(),
        });
    }

    /// Emit a started event.
    pub fn emit_started(&self, name: &str) {
        self.emit(LifecycleEvent::Started {
            name: name.to_string(),
            at: Instant::now(),
        });
    }

    /// Emit a stopped event.
    pub fn emit_stopped(&self, name: &str) {
        self.emit(LifecycleEvent::Stopped {
            name: name.to_string(),
            at: Instant::now(),
        });
    }

    /// Emit a reloaded event.
    pub fn emit_reloaded(&self, name: &str, count: u64) {
        self.emit(LifecycleEvent::Reloaded {
            name: name.to_string(),
            at: Instant::now(),
            count,
        });
    }

    /// Emit an unloaded event.
    pub fn emit_unloaded(&self, name: &str) {
        self.emit(LifecycleEvent::Unloaded {
            name: name.to_string(),
            at: Instant::now(),
        });
    }

    /// Emit an error event.
    pub fn emit_error(&self, name: &str, message: &str) {
        self.emit(LifecycleEvent::Error {
            name: name.to_string(),
            message: message.to_string(),
            at: Instant::now(),
        });
    }
}

impl Default for LifecycleHooks {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for LifecycleHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LifecycleHooks")
            .field("handler_count", &self.handlers.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_lifecycle_state_transitions() {
        assert!(LifecycleState::Initialized.can_start());
        assert!(!LifecycleState::Created.can_start());

        assert!(LifecycleState::Running.can_stop());
        assert!(!LifecycleState::Stopped.can_stop());

        assert!(LifecycleState::Running.can_call());
        assert!(!LifecycleState::Stopped.can_call());

        assert!(LifecycleState::Running.can_reload());
        assert!(!LifecycleState::Unloaded.can_reload());

        assert!(LifecycleState::Unloaded.is_terminal());
        assert!(!LifecycleState::Running.is_terminal());
    }

    #[test]
    fn test_lifecycle_hooks() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut hooks = LifecycleHooks::new();
        hooks.on_event(move |_| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        hooks.emit_created("test");
        hooks.emit_started("test");
        hooks.emit_stopped("test");

        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_lifecycle_event_info() {
        let event = LifecycleEvent::Started {
            name: "test-plugin".to_string(),
            at: Instant::now(),
        };

        assert_eq!(event.plugin_name(), "test-plugin");
        assert_eq!(event.event_name(), "started");
    }
}
