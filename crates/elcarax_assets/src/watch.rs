use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetWatchEvent {
    pub kind: AssetWatchEventKind,
    pub paths: Vec<PathBuf>,
}

impl AssetWatchEvent {
    pub fn synthetic_change(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: AssetWatchEventKind::Changed,
            paths: vec![path.into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetWatchEventKind {
    Created,
    Modified,
    Removed,
    Changed,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetWatchStatus {
    Stopped,
    Watching(PathBuf),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetWatchError {
    StartFailed(String),
    WatchFailed(String),
}

impl fmt::Display for AssetWatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartFailed(message) => {
                write!(formatter, "asset watcher start failed: {message}")
            }
            Self::WatchFailed(message) => write!(formatter, "asset watcher failed: {message}"),
        }
    }
}

impl Error for AssetWatchError {}

pub struct AssetWatchService {
    watcher: Option<RecommendedWatcher>,
    receiver: Receiver<AssetWatchEvent>,
    status: AssetWatchStatus,
}

impl AssetWatchService {
    pub fn start(root: impl AsRef<Path>) -> Result<Self, AssetWatchError> {
        let root = root.as_ref().to_path_buf();
        let (sender, receiver) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                let event = match result {
                    Ok(event) => map_notify_event(event),
                    Err(error) => AssetWatchEvent {
                        kind: AssetWatchEventKind::Error(error.to_string()),
                        paths: Vec::new(),
                    },
                };
                let _ = sender.send(event);
            },
            Config::default(),
        )
        .map_err(|error| AssetWatchError::StartFailed(error.to_string()))?;
        watcher
            .watch(root.as_path(), RecursiveMode::Recursive)
            .map_err(|error| AssetWatchError::WatchFailed(error.to_string()))?;
        Ok(Self {
            watcher: Some(watcher),
            receiver,
            status: AssetWatchStatus::Watching(root),
        })
    }

    pub fn status(&self) -> &AssetWatchStatus {
        &self.status
    }

    pub fn drain_events(&mut self) -> Vec<AssetWatchEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        coalesce_events(events)
    }

    pub fn stop(&mut self) {
        self.watcher = None;
        self.status = AssetWatchStatus::Stopped;
    }
}

fn map_notify_event(event: notify::Event) -> AssetWatchEvent {
    AssetWatchEvent {
        kind: map_notify_kind(&event.kind),
        paths: event.paths,
    }
}

fn map_notify_kind(kind: &EventKind) -> AssetWatchEventKind {
    match kind {
        EventKind::Create(_) => AssetWatchEventKind::Created,
        EventKind::Modify(_) => AssetWatchEventKind::Modified,
        EventKind::Remove(_) => AssetWatchEventKind::Removed,
        _ => AssetWatchEventKind::Changed,
    }
}

fn coalesce_events(events: Vec<AssetWatchEvent>) -> Vec<AssetWatchEvent> {
    if events.len() <= 1 {
        return events;
    }
    let mut paths = BTreeSet::new();
    let mut has_error = None;
    for event in events {
        if let AssetWatchEventKind::Error(message) = event.kind {
            has_error = Some(message);
        }
        paths.extend(event.paths);
    }
    let kind = match has_error {
        Some(message) => AssetWatchEventKind::Error(message),
        None => AssetWatchEventKind::Changed,
    };
    vec![AssetWatchEvent {
        kind,
        paths: paths.into_iter().collect(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_event_has_path() {
        let event = AssetWatchEvent::synthetic_change("assets/hero.glb");
        assert_eq!(event.kind, AssetWatchEventKind::Changed);
        assert_eq!(event.paths, vec![PathBuf::from("assets/hero.glb")]);
    }

    #[test]
    fn coalescing_merges_paths() {
        let events = coalesce_events(vec![
            AssetWatchEvent::synthetic_change("assets/a.txt"),
            AssetWatchEvent::synthetic_change("assets/b.txt"),
        ]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].paths.len(), 2);
    }
}
