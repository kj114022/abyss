//! Watch mode for incremental context updates
//!
//! Monitors file changes and regenerates context incrementally.

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

/// Watch event for file changes
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// File was modified
    Modified(std::path::PathBuf),
    /// File was created
    Created(std::path::PathBuf),
    /// File was deleted
    Deleted(std::path::PathBuf),
    /// Error occurred
    Error(String),
}

/// File watcher for incremental updates
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<WatchEvent>,
}

impl FileWatcher {
    /// Create a new file watcher for the given path
    pub fn new(path: &Path) -> Result<Self, String> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |result: Result<notify::Event, notify::Error>| match result {
                Ok(event) => {
                    for path in event.paths {
                        let watch_event = match event.kind {
                            notify::EventKind::Modify(_) => WatchEvent::Modified(path),
                            notify::EventKind::Create(_) => WatchEvent::Created(path),
                            notify::EventKind::Remove(_) => WatchEvent::Deleted(path),
                            _ => continue,
                        };
                        let _ = tx.send(watch_event);
                    }
                }
                Err(e) => {
                    let _ = tx.send(WatchEvent::Error(e.to_string()));
                }
            },
            Config::default(),
        )
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path: {}", e))?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    /// Get the next watch event (blocking with timeout)
    pub fn next_event(&self, timeout: Duration) -> Option<WatchEvent> {
        self.receiver.recv_timeout(timeout).ok()
    }

    /// Get all pending events (non-blocking)
    pub fn pending_events(&self) -> Vec<WatchEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }
}

/// Debounce file events to avoid too frequent updates
pub struct Debouncer {
    last_events: std::collections::HashMap<std::path::PathBuf, std::time::Instant>,
    delay: Duration,
}

impl Debouncer {
    /// Create a new debouncer with the given delay
    pub fn new(delay: Duration) -> Self {
        Self {
            last_events: std::collections::HashMap::new(),
            delay,
        }
    }

    /// Check if an event should be processed (not debounced)
    pub fn should_process(&mut self, path: &Path) -> bool {
        let now = std::time::Instant::now();
        if let Some(last) = self.last_events.get(path)
            && now.duration_since(*last) < self.delay
        {
            return false;
        }
        self.last_events.insert(path.to_path_buf(), now);
        true
    }

    /// Clear old entries to prevent memory growth
    pub fn cleanup(&mut self) {
        let now = std::time::Instant::now();
        self.last_events
            .retain(|_, last| now.duration_since(*last) < self.delay * 10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debouncer() {
        let mut debouncer = Debouncer::new(Duration::from_millis(100));
        let path = Path::new("test.rs");

        // First event should pass
        assert!(debouncer.should_process(path));

        // Immediate second event should be debounced
        assert!(!debouncer.should_process(path));

        // After delay, should pass again
        std::thread::sleep(Duration::from_millis(150));
        assert!(debouncer.should_process(path));
    }
}
