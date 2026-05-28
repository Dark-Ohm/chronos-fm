use super::types::{SearchFileResult, SearchMatch, StatusLevel};
use super::ExplorerPage;
use crate::services::fs::listing::FileEntryDto;
use crate::services::search::SearchResult;
use gpui::{AsyncApp, Context, Window};
use std::collections::HashMap;

impl ExplorerPage {
    pub(crate) fn trigger_search(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Invalidate any in-flight search; only the latest request may apply its
        // results (including the empty-query and degraded-mode early returns,
        // which clear results and must not be overwritten by a slower search).
        self.search_generation = self.search_generation.wrapping_add(1);
        let generation = self.search_generation;

        if self.search_query.is_empty() {
            self.search_results = None;
            self.apply_filter();
            self.clear_status();
            cx.notify();
            return;
        }

        let Some(service) = self.search_service.clone() else {
            // Degraded mode: full-text search is unavailable, so fall back to
            // filtering the already-loaded directory by filename.
            self.search_results = None;
            self.apply_filter();
            self.set_status(
                StatusLevel::Error,
                "Search index unavailable; filtering current folder by name only",
            );
            cx.notify();
            return;
        };

        self.is_performing_search = true;
        self.clear_status();
        cx.notify();

        let query = self.search_query.clone();
        let scope = self.search_scope;

        cx.spawn(
            move |this: gpui::WeakEntity<ExplorerPage>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let results = service.search(query, scope).await;

                    this.update(&mut cx, |this, cx| {
                        // Discard results if a newer search has since been issued.
                        if this.search_generation != generation {
                            return;
                        }
                        match results {
                            Ok(res) => {
                                let grouped = group_results(res);
                                let entries = results_to_entries(&grouped);

                                this.filtered_entries = entries;
                                this.search_results = Some(grouped);
                                this.clear_status();
                            }
                            Err(e) => {
                                tracing::error!("Search failed: {}", e);
                                this.search_results = Some(Vec::new());
                                this.filtered_entries = Vec::new();
                                this.set_status(
                                    StatusLevel::Error,
                                    format!("Search failed: {}", e),
                                );
                            }
                        }
                        this.is_performing_search = false;
                        this.update_item_sizes();
                        cx.notify();
                    })
                }
            },
        )
        .detach();
    }

    pub(crate) fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_visible {
            return;
        }
        self.search_visible = true;
        self.search_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub(crate) fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.search_visible {
            return;
        }
        self.search_visible = false;
        self.search_results = None;
        self.search_query.clear();
        self.apply_filter();
        self.update_editor_search(window, cx);
        cx.notify();
    }

    pub(crate) fn toggle_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_visible {
            self.close_search(window, cx);
        } else {
            self.open_search(window, cx);
        }
    }
}

pub fn group_results(results: Vec<SearchResult>) -> Vec<SearchFileResult> {
    let mut file_map: HashMap<String, SearchFileResult> = HashMap::new();

    for res in results {
        let entry = file_map
            .entry(res.path.to_string_lossy().to_string())
            .or_insert_with(|| {
                let path = std::path::Path::new(&res.path);
                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let folder = if let Some(parent) = path.parent() {
                    parent.to_string_lossy().to_string()
                } else {
                    String::new()
                };

                SearchFileResult {
                    path: res.path.to_string_lossy().to_string(),
                    folder,
                    filename,
                    matches: Vec::new(),
                }
            });

        if res.line_number > 0 {
            entry.matches.push(SearchMatch {
                line_number: res.line_number,
                line_content: res.line_content,
                match_start: 0,
                match_end: 0,
            });
        }
    }

    let mut sorted_results: Vec<SearchFileResult> = file_map.into_values().collect();
    sorted_results.sort_by(|a, b| a.path.cmp(&b.path));

    if std::env::var("NOHR_DEBUG").is_ok() {
        for r in &sorted_results {
            tracing::info!(
                "[DEBUG] group_results: file='{}', matches={}",
                r.filename,
                r.matches.len()
            );
        }
    }

    sorted_results
}

pub fn results_to_entries(results: &[SearchFileResult]) -> Vec<FileEntryDto> {
    results
        .iter()
        .map(|res| {
            let meta = std::fs::metadata(&res.path).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            FileEntryDto {
                name: if res.folder.is_empty() {
                    res.filename.clone()
                } else {
                    format!("{}/{}", res.folder, res.filename)
                },
                path: res.path.clone(),
                kind: if is_dir {
                    "dir".to_string()
                } else {
                    "file".to_string()
                },
                size,
                modified,
            }
        })
        .collect()
}
