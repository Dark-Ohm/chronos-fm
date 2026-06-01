pub mod backend;
pub mod engine;
pub mod indexer;
pub mod ripgrep;
pub mod spotlight;
pub mod watcher;

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    Home,
    Root,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
}

pub use backend::SearchBackend;
pub use engine::InitialIndexingJob;

use anyhow::Result;
use std::sync::Arc;

pub struct SearchService {
    engine: Arc<engine::SearchEngine>,
}

impl SearchService {
    pub fn new() -> Result<Self> {
        let engine = Arc::new(engine::SearchEngine::new()?);
        Ok(Self { engine })
    }

    /// Search is synchronous; run it on GPUI's background executor
    /// (`cx.background_spawn`) so the UI thread is not blocked.
    pub fn search(&self, query: String, scope: SearchScope) -> Result<Vec<SearchResult>> {
        self.engine.search(query, scope)
    }

    /// Hands off the one-shot initial-indexing job for the caller to run on a
    /// background executor. Returns `None` once it has been taken.
    pub fn take_initial_indexing_job(&self) -> Option<InitialIndexingJob> {
        self.engine.take_initial_indexing_job()
    }

    pub fn progress_subscription(&self) -> tokio::sync::watch::Receiver<f32> {
        self.engine.progress_subscription()
    }
}
