use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::Project;

const DEFAULT_RECENT_LIMIT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentProjectEntry {
    pub name: String,
    pub path: PathBuf,
}

impl RecentProjectEntry {
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
        }
    }

    pub fn from_project(project: &Project) -> Self {
        Self {
            name: project.name().as_str().to_owned(),
            path: project.root().as_path().to_path_buf(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecentProjectsError {
    Io(String),
    Invalid(String),
}

impl fmt::Display for RecentProjectsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "recent projects IO error: {message}"),
            Self::Invalid(message) => {
                write!(formatter, "recent projects file is invalid: {message}")
            }
        }
    }
}

impl Error for RecentProjectsError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentProjectsStore {
    store_path: PathBuf,
    entries: Vec<RecentProjectEntry>,
    max_entries: usize,
}

impl RecentProjectsStore {
    pub fn new(store_path: impl Into<PathBuf>, max_entries: usize) -> Self {
        Self {
            store_path: store_path.into(),
            entries: Vec::new(),
            max_entries: max_entries.max(1),
        }
    }

    pub fn load(store_path: impl Into<PathBuf>) -> Result<Self, RecentProjectsError> {
        let store_path = store_path.into();
        if !store_path.is_file() {
            return Ok(Self::new(store_path, DEFAULT_RECENT_LIMIT));
        }
        let content = fs::read_to_string(&store_path)
            .map_err(|error| RecentProjectsError::Io(error.to_string()))?;
        let document: RecentProjectsDocument = toml::from_str(&content)
            .map_err(|error| RecentProjectsError::Invalid(error.to_string()))?;
        Ok(Self {
            store_path,
            entries: document
                .entries
                .into_iter()
                .map(RecentProjectEntryDocument::into_entry)
                .collect(),
            max_entries: document.max_entries.unwrap_or(DEFAULT_RECENT_LIMIT).max(1),
        })
    }

    pub fn save(&self) -> Result<(), RecentProjectsError> {
        if let Some(parent) = self.store_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| RecentProjectsError::Io(error.to_string()))?;
        }
        let document = RecentProjectsDocument {
            max_entries: Some(self.max_entries),
            entries: self
                .entries
                .iter()
                .map(RecentProjectEntryDocument::from_entry)
                .collect(),
        };
        let content = toml::to_string_pretty(&document)
            .map_err(|error| RecentProjectsError::Invalid(error.to_string()))?;
        fs::write(&self.store_path, content)
            .map_err(|error| RecentProjectsError::Io(error.to_string()))
    }

    pub fn add(&mut self, entry: RecentProjectEntry) {
        let canonical = canonical_path(&entry.path);
        self.entries
            .retain(|existing| canonical_path(&existing.path) != canonical);
        self.entries.insert(0, entry);
        self.entries.truncate(self.max_entries);
    }

    pub fn add_project(&mut self, project: &Project) {
        self.add(RecentProjectEntry::from_project(project));
    }

    pub fn most_recent(&self) -> Option<&RecentProjectEntry> {
        self.entries.first()
    }

    pub fn entries(&self) -> &[RecentProjectEntry] {
        self.entries.as_slice()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn store_path(&self) -> &Path {
        self.store_path.as_path()
    }

    pub fn summary(&self) -> String {
        if self.entries.is_empty() {
            return "No recent projects".to_string();
        }
        self.entries
            .iter()
            .take(5)
            .map(|entry| format!("{} ({})", entry.name, entry.path.display()))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RecentProjectsDocument {
    #[serde(default)]
    max_entries: Option<usize>,
    #[serde(default)]
    entries: Vec<RecentProjectEntryDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RecentProjectEntryDocument {
    name: String,
    path: String,
}

impl RecentProjectEntryDocument {
    fn from_entry(entry: &RecentProjectEntry) -> Self {
        Self {
            name: entry.name.clone(),
            path: entry.path.display().to_string(),
        }
    }

    fn into_entry(self) -> RecentProjectEntry {
        RecentProjectEntry {
            name: self.name,
            path: PathBuf::from(self.path),
        }
    }
}

fn canonical_path(path: &Path) -> PathBuf {
    match fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => path.to_path_buf(),
    }
}

/// In-memory recent project list used by UI snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentProjects {
    entries: Vec<RecentProjectEntry>,
    max_len: usize,
}

impl RecentProjects {
    pub fn new(max_len: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_len: max_len.max(1),
        }
    }

    pub fn from_store(store: &RecentProjectsStore) -> Self {
        Self {
            entries: store.entries().to_vec(),
            max_len: store.max_entries,
        }
    }

    pub fn record(&mut self, project: &Project) {
        let entry = RecentProjectEntry::from_project(project);
        let canonical = canonical_path(&entry.path);
        self.entries
            .retain(|existing| canonical_path(&existing.path) != canonical);
        self.entries.insert(0, entry);
        self.entries.truncate(self.max_len);
    }

    pub fn entries(&self) -> &[RecentProjectEntry] {
        self.entries.as_slice()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for RecentProjects {
    fn default() -> Self {
        Self::new(DEFAULT_RECENT_LIMIT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Project, project_id_from_root};
    use crate::manifest::ResolvedProjectPaths;
    use std::fs;

    fn fixture_project(name: &str, path: PathBuf) -> Project {
        Project::from_loaded_data(
            project_id_from_root(&path),
            name,
            &path,
            ResolvedProjectPaths {
                asset_root: path.join("assets"),
                scene_root: path.join("scenes"),
                settings_dir: path.join(".elcarax"),
            },
        )
    }

    #[test]
    fn add_project_updates_recency() {
        let mut store = RecentProjectsStore::new("ignored", 10);
        let first = fixture_project("First", PathBuf::from("/tmp/first"));
        let second = fixture_project("Second", PathBuf::from("/tmp/second"));
        store.add_project(&first);
        store.add_project(&second);
        store.add_project(&first);
        assert_eq!(store.len(), 2);
        assert_eq!(
            store.most_recent().map(|entry| entry.name.as_str()),
            Some("First")
        );
    }

    #[test]
    fn deduplicate_path_keeps_single_entry() {
        let mut store = RecentProjectsStore::new("ignored", 10);
        let path = PathBuf::from("/tmp/project-a");
        store.add(RecentProjectEntry::new("A", &path));
        store.add(RecentProjectEntry::new("A Renamed", &path));
        assert_eq!(store.len(), 1);
        assert_eq!(store.entries()[0].name, "A Renamed");
    }

    #[test]
    fn cap_list_respects_max_entries() {
        let mut store = RecentProjectsStore::new("ignored", 2);
        store.add(RecentProjectEntry::new("One", "/tmp/one"));
        store.add(RecentProjectEntry::new("Two", "/tmp/two"));
        store.add(RecentProjectEntry::new("Three", "/tmp/three"));
        assert_eq!(store.len(), 2);
        assert_eq!(store.entries()[0].name, "Three");
    }

    #[test]
    fn save_load_round_trip() {
        let temp = std::env::temp_dir().join(format!("elcarax-recent-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = fs::create_dir_all(&temp);
        let store_path = temp.join("recent-projects.toml");
        let mut store = RecentProjectsStore::new(&store_path, 10);
        store.add(RecentProjectEntry::new("Saved", temp.join("project")));
        if let Err(error) = store.save() {
            panic!("save recent projects: {error}");
        }
        let loaded = match RecentProjectsStore::load(&store_path) {
            Ok(store) => store,
            Err(error) => panic!("load recent projects: {error}"),
        };
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.entries()[0].name, "Saved");
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn missing_recent_file_returns_empty_list() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-recent-missing-{}", std::process::id()));
        let store_path = temp.join("missing-recent.toml");
        let store = match RecentProjectsStore::load(&store_path) {
            Ok(store) => store,
            Err(error) => panic!("missing file should load empty: {error}"),
        };
        assert!(store.is_empty());
    }
}
