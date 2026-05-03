use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::Mutex;
use tracing::info;

use crate::index::file_tree::FileTree;
use crate::index::{walker, watcher};
use crate::ops::buffers::Buffer;
use crate::ops::subcalls::SubcallResult;
use crate::server::errors::AppError;
use crate::server::session::Session;
use crate::symbols::{parser, SymbolTable};

/// A single indexed project with its own file tree, symbol table, and watcher.
pub struct Project {
    pub root: PathBuf,
    pub file_tree: Arc<FileTree>,
    pub symbol_table: Arc<SymbolTable>,
    // Held alive to keep the filesystem watcher running; dropped on eviction.
    #[allow(dead_code)]
    pub watcher: Option<watcher::WatcherHandle>,
    pub last_active: Mutex<DateTime<Utc>>,
    /// Named buffers for storing code snippets.
    pub buffers: DashMap<String, Buffer>,
    /// JSON variables for RLM state management.
    pub variables: DashMap<String, serde_json::Value>,
    /// Subcall results from recursive LLM queries.
    pub subcall_results: Mutex<Vec<SubcallResult>>,
}

/// Shared application state, wrapped in Arc for axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    pub projects: DashMap<PathBuf, Arc<Project>>,
    pub sessions: DashMap<String, Session>,
    pub max_projects: usize,
    pub max_file_size: u64,
}

impl AppState {
    pub fn new(max_projects: usize, max_file_size: u64) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                projects: DashMap::new(),
                sessions: DashMap::new(),
                max_projects,
                max_file_size,
            }),
        }
    }

    /// Look up an existing project or index a new one. Evicts LRU if at capacity.
    pub fn get_or_create_project(&self, cwd: &Path) -> Result<Arc<Project>, AppError> {
        let canonical = cwd.canonicalize().map_err(|e| {
            AppError::BadRequest(format!("Path not accessible: {}", e))
        })?;

        if !canonical.is_dir() {
            return Err(AppError::BadRequest(format!(
                "'{}' is not a directory",
                canonical.display()
            )));
        }

        // Return existing project if found
        if let Some(project) = self.inner.projects.get(&canonical) {
            *project.last_active.lock() = Utc::now();
            return Ok(project.clone());
        }

        // Check capacity, evict if needed
        if self.inner.projects.len() >= self.inner.max_projects {
            self.evict_lru()?;
        }

        // Scan directory
        let file_tree = Arc::new(FileTree::new());
        let symbol_table = Arc::new(SymbolTable::new());
        let max_file_size = self.inner.max_file_size;

        info!("Indexing new project: {}", canonical.display());
        let file_count =
            walker::scan_directory(&canonical, &file_tree, max_file_size)
                .map_err(|e| AppError::Internal(e.to_string()))?;
        info!("Indexed {} files for {}", file_count, canonical.display());

        // Start watcher
        let watcher_handle = watcher::start_watcher(
            &canonical,
            file_tree.clone(),
            symbol_table.clone(),
            max_file_size,
        )
        .ok();

        let project = Arc::new(Project {
            root: canonical.clone(),
            file_tree: file_tree.clone(),
            symbol_table: symbol_table.clone(),
            watcher: watcher_handle,
            last_active: Mutex::new(Utc::now()),
            buffers: DashMap::new(),
            variables: DashMap::new(),
            subcall_results: Mutex::new(Vec::new()),
        });

        self.inner.projects.insert(canonical, project.clone());

        // Spawn symbol extraction in background
        let ft = file_tree;
        let st = symbol_table;
        let root = project.root.clone();
        tokio::spawn(async move {
            info!("Starting symbol extraction for {}...", root.display());
            match parser::extract_all_symbols(&root, &ft, &st).await {
                Ok(count) => info!("Extracted {} symbols for {}", count, root.display()),
                Err(e) => tracing::error!("Symbol extraction failed for {}: {}", root.display(), e),
            }
        });

        Ok(project)
    }

    /// Look up the project for a given session. Returns a descriptive error if
    /// the project has been evicted.
    pub fn get_project_for_session(&self, session_id: &str) -> Result<Arc<Project>, AppError> {
        let session = self
            .inner
            .sessions
            .get(session_id)
            .ok_or_else(|| AppError::NotFound(format!("Session '{}' not found", session_id)))?;

        let project_path = &session.project_path;

        let project = self
            .inner
            .projects
            .get(project_path)
            .ok_or_else(|| {
                AppError::Gone(format!(
                    "Project at '{}' was evicted due to capacity limits. \
                     Start a new session to re-index, or increase --max-projects.",
                    project_path.display()
                ))
            })?;

        Ok(project.clone())
    }

    /// Update the last-active timestamp on a project.
    pub fn touch_project(&self, project_path: &Path) {
        if let Some(project) = self.inner.projects.get(project_path) {
            *project.last_active.lock() = Utc::now();
        }
    }

    /// Evict the least recently used project. Removes all sessions pointing to it.
    fn evict_lru(&self) -> Result<(), AppError> {
        // Find the project with the oldest last_active
        let oldest = self
            .inner
            .projects
            .iter()
            .min_by_key(|entry| *entry.value().last_active.lock())
            .map(|entry| entry.key().clone());

        let path = oldest.ok_or_else(|| {
            AppError::Internal("No projects to evict".into())
        })?;

        info!("Evicting project: {}", path.display());

        // Remove the project (drops watcher)
        self.inner.projects.remove(&path);

        // Remove all sessions attached to this project
        self.inner.sessions.retain(|_, session| session.project_path != path);

        Ok(())
    }
}
