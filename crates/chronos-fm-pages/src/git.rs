//! Live Git status panel (T010, Milestone A + B).
//!
//! Three sections (Staged / Modified / Untracked) with per-row stage/unstage
//! actions, a commit bar, directory follow/pin, **local branch list /
//! create / checkout**, and a **unified text diff** for the selected path
//! (Milestone B).
//!
//! Repository status is computed on the background executor and refreshed by a
//! `.git`-directory watcher (400 ms debounce), following the same channel +
//! foreground-poll pattern as `config.toml` hot reload in `root.rs`.

use crate::explorer::ExplorerPage;
use chronos_fm_services::git::watcher::GitWatcher;
use chronos_fm_services::git::{
    self, CommitDetail, CommitEntry, GitError, RemoteEntry, RepoStatus, StashEntry,
};
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::input::Input;
use gpui_component::input::InputState;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

/// Resolve the directory the Git panel should display: the pinned path when
/// set, otherwise the active explorer tab's directory (follow mode). Returns
/// `None` when neither is available. Extracted for unit testing (T017).
fn resolve_follow_dir(
    pinned_path: Option<&std::path::Path>,
    explorer_path: Option<&str>,
) -> Option<PathBuf> {
    if let Some(p) = pinned_path {
        return Some(p.to_path_buf());
    }
    explorer_path.map(PathBuf::from)
}

/// Whether the refresh loop should trigger a reload: either a git-content
/// change signal arrived (`changed`), or the follow-mode directory moved —
/// which is how the panel tracks explorer navigation (T017).
fn should_refresh(changed: bool, follow_dir: Option<&PathBuf>, last_dir: Option<&PathBuf>) -> bool {
    changed || follow_dir != last_dir
}

/// Selection for the unified-diff panel (Milestone B).
#[derive(Clone, Debug)]
struct DiffSelection {
    path: String,
    /// True when the path is shown under Staged (index vs HEAD).
    staged: bool,
    text: String,
}

/// Which of the Git tab's five sub-views is active (T038, mirrors the
/// mockup's `changes | history | branches | stashes | remotes` sidebar nav).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GitView {
    /// Staged/unstaged/untracked file groups + commit/amend box + diff pane.
    #[default]
    Changes,
    /// Bounded commit log + selected-commit detail.
    History,
    /// Local branch list/create/checkout.
    Branches,
    /// Stash push/pop/apply/drop.
    Stashes,
    /// Configured remotes + fetch/push/delete + Add Remote form.
    Remotes,
}

impl GitView {
    fn label(self) -> &'static str {
        match self {
            GitView::Changes => "Changes",
            GitView::History => "History",
            GitView::Branches => "Branches",
            GitView::Stashes => "Stashes",
            GitView::Remotes => "Remotes",
        }
    }

    /// Parses a sub-view name from `--page=git:<name>` (T046 residual,
    /// case-insensitive). Mirrors `PageKind::from_cli_name`.
    fn from_cli_name(name: &str) -> Option<GitView> {
        match name.trim().to_lowercase().as_str() {
            "changes" => Some(GitView::Changes),
            "history" => Some(GitView::History),
            "branches" => Some(GitView::Branches),
            "stashes" => Some(GitView::Stashes),
            "remotes" => Some(GitView::Remotes),
            _ => None,
        }
    }
}

/// How many commits `history` bounds a single `git log` read to (T038).
const HISTORY_LIMIT: usize = 50;

/// The live Git status panel.
pub struct GitPage {
    explorer: WeakEntity<ExplorerPage>,
    pinned_path: Option<PathBuf>,
    status: Option<RepoStatus>,
    no_repo: bool,
    error: Option<String>,
    refreshing: bool,
    message_input: Entity<InputState>,
    /// Milestone B: new-branch name field.
    branch_input: Entity<InputState>,
    /// Local branch names (short).
    branches: Vec<String>,
    /// Selected file unified diff (if any).
    selected_diff: Option<DiffSelection>,
    /// C6: prevents double-click on push/pull/stash operations.
    busy: bool,
    /// Stash entries from the latest refresh.
    stashes: Vec<chronos_fm_services::git::StashEntry>,
    /// Stash message input.
    stash_input: Entity<InputState>,
    _watcher: Option<GitWatcher>,
    _shutdown_tx: Option<mpsc::Sender<()>>,
    refresh_tx: Option<mpsc::Sender<()>>,
    /// The directory the panel last refreshed in follow mode. The refresh
    /// loop compares it against the current follow dir every tick, so explorer
    /// navigation (which emits no signal into the watcher channel) still
    /// triggers a reload (T017).
    last_dir: Option<PathBuf>,
    /// Monotonic counter of refresh starts; results from an in-flight status
    /// read that are older than the latest start are discarded, so a slow
    /// stale read cannot overwrite newer state after rapid follow-mode
    /// navigation (T017).
    refresh_generation: u64,

    // --- T038: sub-navigation + History/Remotes/amend ----------------------
    /// Active sub-view (Changes/History/Branches/Stashes/Remotes).
    view: GitView,
    /// Bounded commit history, refreshed alongside status.
    history: Vec<CommitEntry>,
    /// Full hash of the commit selected in History, if any.
    selected_commit: Option<String>,
    /// Detail (metadata + per-file stats) for `selected_commit`, loaded
    /// on-demand when a commit row is clicked.
    commit_detail: Option<CommitDetail>,
    /// Configured remotes, refreshed alongside status.
    remotes: Vec<RemoteEntry>,
    /// Whether the commit box is in Amend mode (Changes view segmented
    /// control).
    amend: bool,
    /// Add Remote form: name field.
    remote_name_input: Entity<InputState>,
    /// Add Remote form: URL field.
    remote_url_input: Entity<InputState>,
}

impl GitPage {
    /// Create the live Git panel. `explorer` is followed for the directory to
    /// display unless the user pins one; the commit-message input and the
    /// refresh loop (`.git` watcher, 400 ms debounce) are started here.
    pub fn new(
        explorer: WeakEntity<ExplorerPage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let message_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Commit message", window, cx);
            state
        });
        let branch_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("New branch name", window, cx);
            state
        });

        let stash_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Stash message", window, cx);
            state
        });
        let remote_name_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("name (e.g. upstream)", window, cx);
            state
        });
        let remote_url_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("url", window, cx);
            state
        });

        let (tx, rx) = mpsc::channel::<()>();
        let refresh_tx = tx.clone();

        let mut page = Self {
            explorer,
            pinned_path: None,
            status: None,
            no_repo: false,
            error: None,
            refreshing: false,
            message_input,
            branch_input,
            branches: Vec::new(),
            selected_diff: None,
            busy: false,
            stashes: Vec::new(),
            stash_input,
            _watcher: None,
            _shutdown_tx: Some(tx),
            refresh_tx: Some(refresh_tx),
            last_dir: None,
            refresh_generation: 0,
            view: GitView::default(),
            history: Vec::new(),
            selected_commit: None,
            commit_detail: None,
            remotes: Vec::new(),
            amend: false,
            remote_name_input,
            remote_url_input,
        };
        page.start_refresh_loop(window, rx, cx);
        page
    }

    /// Reload the repository status for the current directory (follow mode or
    /// pin). Called by the refresh loop, the Refresh button, and after every
    /// stage/unstage/commit.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        self.last_dir = dir.clone();
        self.do_refresh(dir, cx);
    }

    fn current_dir(&self, cx: &mut Context<Self>) -> Option<PathBuf> {
        let explorer_path = self
            .explorer
            .upgrade()
            .map(|explorer| explorer.read(cx).current_path(cx));
        resolve_follow_dir(self.pinned_path.as_deref(), explorer_path.as_deref())
    }

    fn do_refresh(&mut self, dir: Option<PathBuf>, cx: &mut Context<Self>) {
        let Some(dir) = dir else { return };
        self.refreshing = true;
        // Follow-mode navigation can start a new read while an older one is
        // still in flight; bump the generation so the older result is dropped
        // when it lands (T017).
        self.refresh_generation += 1;
        let generation = self.refresh_generation;
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let repo = git::open_repo(&dir)?;
                        let status = git::status(&repo)?;
                        let branches = git::list_branches(&repo).unwrap_or_default();
                        let stashes = git::stash_list(&repo).unwrap_or_default();
                        // T038: History and Remotes are read best-effort — a
                        // repo with no commits (empty history) or no
                        // configured remotes is not an error, matches
                        // `unwrap_or_default()` already used for
                        // branches/stashes above.
                        let history = git::history(&repo, HISTORY_LIMIT).unwrap_or_default();
                        let remotes = git::remotes(&repo).unwrap_or_default();
                        Ok::<_, GitError>((status, branches, stashes, history, remotes))
                    })
                    .await;
                this.update(&mut cx, |page, cx| {
                    if page.refresh_generation != generation {
                        // A newer refresh superseded this one; drop the stale
                        // result entirely (its `refreshing` flag is owned by
                        // the newer read).
                        return;
                    }
                    page.refreshing = false;
                    match result {
                        Ok((status, branches, stashes, history, remotes)) => {
                            page.recreate_watcher(&status.git_dir);
                            page.status = Some(status);
                            page.branches = branches;
                            page.stashes = stashes;
                            // Keep the selected commit's detail in sync (a
                            // refresh can happen while History is open); drop
                            // the selection if that commit no longer exists
                            // in the bounded window instead of showing stale
                            // detail for a hash that scrolled out.
                            if let Some(hash) = &page.selected_commit {
                                if !history.iter().any(|c| &c.full_hash == hash) {
                                    page.selected_commit = None;
                                    page.commit_detail = None;
                                }
                            }
                            page.history = history;
                            page.remotes = remotes;
                            page.no_repo = false;
                            page.error = None;
                        }
                        Err(GitError::NotARepository) => {
                            page.status = None;
                            page.branches.clear();
                            page.selected_diff = None;
                            page.history.clear();
                            page.remotes.clear();
                            page.selected_commit = None;
                            page.commit_detail = None;
                            page.no_repo = true;
                            page.error = None;
                            page._watcher = None;
                        }
                        Err(e) => {
                            page.error = Some(e.to_string());
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn recreate_watcher(&mut self, git_dir: &std::path::Path) {
        let git_dir_owned = git_dir.to_path_buf();
        if let Some(tx) = self.refresh_tx.clone() {
            self._watcher = GitWatcher::new(&git_dir_owned, move || {
                tx.send(()).ok();
            })
            .ok();
        }
    }

    fn start_refresh_loop(
        &mut self,
        window: &mut Window,
        rx: mpsc::Receiver<()>,
        cx: &mut Context<Self>,
    ) {
        // Initial refresh
        self.refresh(cx);

        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(400))
                            .await;
                        let mut changed = false;
                        let mut disconnected = false;
                        loop {
                            match rx.try_recv() {
                                Ok(()) => changed = true,
                                Err(mpsc::TryRecvError::Empty) => break,
                                Err(mpsc::TryRecvError::Disconnected) => {
                                    disconnected = true;
                                    break;
                                }
                            }
                        }
                        if disconnected {
                            break;
                        }
                        // Re-read the follow-mode directory every tick: explorer
                        // navigation emits no signal into this channel, so
                        // `current_dir != last_dir` is how the panel follows the
                        // active tab (T017). The watcher's `changed` signal still
                        // drives refreshes for git-content mutations in place.
                        if this
                            .update_in(&mut cx, |page, _w, cx| {
                                let dir = page.current_dir(cx);
                                if should_refresh(changed, dir.as_ref(), page.last_dir.as_ref()) {
                                    page.refresh(cx);
                                }
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            },
        )
        .detach();
    }

    // --- actions (called from UI listeners with `window` from the event) ---

    fn stage(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        let path = path.to_string();
        let err_path = path.clone();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::stage_path(&repo, &path).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        if let Err(msg) = result {
                            page.error = Some(format!("stage {err_path}: {msg}"));
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn unstage(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        let path = path.to_string();
        let err_path = path.clone();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::unstage_path(&repo, &path).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        if let Err(msg) = result {
                            page.error = Some(format!("unstage {err_path}: {msg}"));
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        let message = self.message_input.read(cx).text().to_string();
        let amend = self.amend;
        // T038: an empty message is only valid for Amend (keeps the original
        // commit's message unchanged) — a plain commit always needs a
        // subject line.
        if message.trim().is_empty() && !amend {
            self.error = Some("commit message is empty".to_string());
            cx.notify();
            return;
        }
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            if amend {
                                git::commit_amend(&repo, &message).map_err(|e| e.to_string())
                            } else {
                                git::commit(&repo, &message).map_err(|e| e.to_string())
                            }
                        })
                        .await;
                    this.update_in(&mut cx, |page, window, cx| {
                        if let Err(msg) = result {
                            page.error =
                                Some(format!("{}: {msg}", if amend { "amend" } else { "commit" }));
                        } else {
                            page.message_input.update(cx, |input, cx| {
                                input.set_value("", window, cx);
                            });
                            page.amend = false;
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn toggle_pin(&mut self, cx: &mut Context<Self>) {
        if self.pinned_path.is_some() {
            self.pinned_path = None;
        } else if let Some(s) = &self.status {
            self.pinned_path = Some(s.workdir.clone());
        }
        self._watcher = None;
        self.refresh(cx);
    }

    // --- Milestone B -------------------------------------------------------

    fn checkout_branch(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        let name = name.to_string();
        let err_name = name.clone();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::checkout_branch(&repo, &name).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        if let Err(msg) = result {
                            page.error = Some(format!("checkout {err_name}: {msg}"));
                        } else {
                            page.selected_diff = None;
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn create_branch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        let name = self.branch_input.read(cx).text().to_string();
        if name.trim().is_empty() {
            self.error = Some("branch name is empty".to_string());
            cx.notify();
            return;
        }
        let err_name = name.clone();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::create_branch(&repo, &name).map_err(|e| e.to_string())?;
                            git::checkout_branch(&repo, &name).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, window, cx| {
                        if let Err(msg) = result {
                            page.error = Some(format!("create branch {err_name}: {msg}"));
                        } else {
                            page.branch_input.update(cx, |input, cx| {
                                input.set_value("", window, cx);
                            });
                            page.selected_diff = None;
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn show_diff(&mut self, path: &str, staged: bool, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        let path = path.to_string();
        let err_path = path.clone();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::unified_diff(&repo, &path, staged).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        match result {
                            Ok(text) => {
                                page.selected_diff = Some(DiffSelection {
                                    path: err_path,
                                    staged,
                                    text,
                                });
                                page.error = None;
                            }
                            Err(msg) => {
                                page.error = Some(format!("diff {err_path}: {msg}"));
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    // --- T038: sub-nav, History, Remotes, amend -----------------------------

    fn select_view(&mut self, view: GitView, cx: &mut Context<Self>) {
        self.view = view;
        cx.notify();
    }

    /// `--page=git:<name>` (T046 residual): select a sub-view by CLI name
    /// at startup. Silently warns and leaves the default view on an
    /// unrecognized name — never aborts the launch over a typo.
    pub(crate) fn set_initial_subview(&mut self, name: &str, cx: &mut Context<Self>) {
        match GitView::from_cli_name(name) {
            Some(view) => self.select_view(view, cx),
            None => tracing::warn!("ignoring unrecognized git sub-view {name:?}"),
        }
    }

    fn toggle_amend(&mut self, cx: &mut Context<Self>) {
        self.amend = !self.amend;
        cx.notify();
    }

    /// Select a commit in History and load its detail (metadata + per-file
    /// stats) on demand.
    fn select_commit(&mut self, hash: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_commit = Some(hash.to_string());
        self.commit_detail = None;
        cx.notify();
        let dir = self.current_dir(cx);
        let hash = hash.to_string();
        let err_hash = hash.clone();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::commit_detail(&repo, &hash).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        // The user may have selected a different commit (or
                        // navigated away) while this background read was in
                        // flight; only apply it if it's still the active
                        // selection.
                        if page.selected_commit.as_deref() != Some(err_hash.as_str()) {
                            return;
                        }
                        match result {
                            Ok(detail) => {
                                page.commit_detail = Some(detail);
                                page.error = None;
                            }
                            Err(msg) => {
                                page.error = Some(format!("commit detail {err_hash}: {msg}"));
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn remote_fetch(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        let name = name.to_string();
        let err_name = name.clone();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::fetch(&repo, &name).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        page.busy = false;
                        if let Err(msg) = result {
                            page.error = Some(format!("fetch {err_name}: {msg}"));
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn remote_delete(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        let name = name.to_string();
        let err_name = name.clone();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::delete_remote(&repo, &name).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        page.busy = false;
                        if let Err(msg) = result {
                            page.error = Some(format!("remote delete {err_name}: {msg}"));
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn add_remote_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let name = self.remote_name_input.read(cx).text().to_string();
        let url = self.remote_url_input.read(cx).text().to_string();
        if name.trim().is_empty() || url.trim().is_empty() {
            self.error = Some("remote name and url are both required".to_string());
            cx.notify();
            return;
        }
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        let err_name = name.clone();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::add_remote(&repo, &name, &url).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, window, cx| {
                        page.busy = false;
                        if let Err(msg) = result {
                            page.error = Some(format!("add remote {err_name}: {msg}"));
                        } else {
                            page.remote_name_input.update(cx, |input, cx| {
                                input.set_value("", window, cx);
                            });
                            page.remote_url_input.update(cx, |input, cx| {
                                input.set_value("", window, cx);
                            });
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }
}

// --- render ----------------------------------------------------------------

impl crate::Page for GitPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.render_body(cx).into_any_element()
    }
}

impl Render for GitPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_body(cx)
    }
}

impl GitPage {
    // --- Milestone C: push / pull / stash ---------------------------------

    fn push(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        } // C6
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::push(&repo, "origin").map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        page.busy = false;
                        if let Err(msg) = result {
                            page.error = Some(format!("push: {msg}"));
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn pull(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        } // C6
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::pull(&repo, "origin").map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        page.busy = false;
                        if let Err(msg) = result {
                            page.error = Some(format!("pull: {msg}"));
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn stash_push_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        let message = self.stash_input.read(cx).text().to_string();
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::stash_push(&repo, &message).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, window, cx| {
                        page.busy = false;
                        if let Err(msg) = result {
                            page.error = Some(format!("stash: {msg}"));
                        } else {
                            page.stash_input.update(cx, |input, cx| {
                                input.set_value("", window, cx);
                            });
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn stash_pop(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::stash_pop(&repo, index).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        page.busy = false;
                        if let Err(msg) = result {
                            page.error = Some(format!("stash pop: {msg}"));
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn stash_drop(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        cx.spawn_in(
            window,
            move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                            let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                            git::stash_drop(&repo, index).map_err(|e| e.to_string())
                        })
                        .await;
                    this.update_in(&mut cx, |page, _window, cx| {
                        page.busy = false;
                        if let Err(msg) = result {
                            page.error = Some(format!("stash drop: {msg}"));
                        }
                        page.refresh(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn render_body(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.status.clone();
        let no_repo = self.no_repo;
        let error = self.error.clone();
        let refreshing = self.refreshing;
        let branches = self.branches.clone();
        let current_branch = status
            .as_ref()
            .map(|s| s.branch.clone())
            .unwrap_or_default();
        let busy = self.busy;
        let selected_diff = self.selected_diff.clone();
        let branch_input = self.branch_input.clone();
        let stashes = self.stashes.clone();
        let stash_input = self.stash_input.clone();
        let view = self.view;
        let history = self.history.clone();
        let remotes = self.remotes.clone();
        let selected_commit = self.selected_commit.clone();
        let commit_detail = self.commit_detail.clone();
        let amend = self.amend;
        let remote_name_input = self.remote_name_input.clone();
        let remote_url_input = self.remote_url_input.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(px(24.))
            .bg(theme::bg(cx))
            .child(render_header(
                status.as_ref(),
                no_repo,
                error,
                refreshing,
                busy,
                cx,
            ))
            .when_some(status.as_ref(), |el, s| {
                let staged_empty = s.staged.is_empty();
                el.child(render_sub_nav(
                    view,
                    s.staged.len() + s.modified.len() + s.untracked.len(),
                    history.len(),
                    branches.len(),
                    stashes.len(),
                    remotes.len(),
                    cx,
                ))
                .child(match view {
                    GitView::Changes => {
                        changes_view(self, s, selected_diff.as_ref(), staged_empty, amend, cx)
                            .into_any_element()
                    }
                    GitView::History => history_view(
                        &history,
                        selected_commit.as_deref(),
                        commit_detail.as_ref(),
                        cx,
                    )
                    .into_any_element(),
                    GitView::Branches => {
                        render_branches(&branches, &current_branch, branch_input, cx)
                            .into_any_element()
                    }
                    GitView::Stashes => {
                        render_stash_section(&stashes, stash_input, busy, cx).into_any_element()
                    }
                    GitView::Remotes => {
                        remotes_view(&remotes, remote_name_input, remote_url_input, busy, cx)
                            .into_any_element()
                    }
                })
            })
    }
}

/// Compact sub-navigation strip: one chip per [`GitView`], each with an
/// honest count badge derived from already-loaded data (T038 — mirrors the
/// mockup's sidebar nav, as a horizontal strip rather than a left column;
/// see T038 report for the deviation).
#[allow(clippy::too_many_arguments)]
fn render_sub_nav(
    active: GitView,
    changes_count: usize,
    history_count: usize,
    branches_count: usize,
    stashes_count: usize,
    remotes_count: usize,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    let items = [
        (GitView::Changes, changes_count),
        (GitView::History, history_count),
        (GitView::Branches, branches_count),
        (GitView::Stashes, stashes_count),
        (GitView::Remotes, remotes_count),
    ];
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .p(px(4.))
        .rounded(px(10.))
        .bg(theme::bg_secondary(cx))
        .border_1()
        .border_color(theme::border(cx))
        .children(items.into_iter().map(|(view, count)| {
            let is_active = view == active;
            div()
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(6.))
                .px(px(12.))
                .py(px(6.))
                .rounded(px(7.))
                .text_sm()
                .when(is_active, |this| {
                    this.bg(theme::accent(cx))
                        .text_color(theme::bg(cx))
                        .font_weight(gpui::FontWeight::BOLD)
                })
                .when(!is_active, |this| {
                    this.text_color(theme::fg_secondary(cx))
                        .hover(|this| this.bg(theme::bg_hover(cx)))
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _ev, _window, cx| {
                        this.select_view(view, cx);
                    }),
                )
                .child(view.label())
                .when(count > 0, |this| {
                    this.child(
                        div()
                            .px(px(6.))
                            .rounded(px(999.))
                            .text_xs()
                            .when(is_active, |b| {
                                b.bg(theme::bg(cx)).text_color(theme::accent(cx))
                            })
                            .when(!is_active, |b| {
                                b.bg(theme::border(cx)).text_color(theme::fg_secondary(cx))
                            })
                            .child(count.to_string()),
                    )
                })
        }))
}

/// Changes view: file sections + commit/amend box + diff pane (T038 §3 —
/// the segmented Commit/Amend control replaces the plain commit bar).
fn changes_view(
    page: &GitPage,
    s: &RepoStatus,
    selected_diff: Option<&DiffSelection>,
    staged_empty: bool,
    amend: bool,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(render_file_section(
            "Staged",
            &s.staged,
            false,
            false,
            theme::accent(cx),
            cx,
        ))
        .child(render_file_section(
            "Modified",
            &s.modified,
            true,
            false,
            theme::muted(cx),
            cx,
        ))
        .child(render_file_section(
            "Untracked",
            &s.untracked,
            true,
            false,
            theme::fg_secondary(cx),
            cx,
        ))
        .child(render_commit_bar(
            page.message_input.clone(),
            staged_empty,
            amend,
            cx,
        ))
        .when_some(selected_diff, |el, d| el.child(render_diff_panel(d, cx)))
}

/// History view: bounded commit list + selected-commit detail (T038).
fn history_view(
    history: &[CommitEntry],
    selected: Option<&str>,
    detail: Option<&CommitDetail>,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    if history.is_empty() {
        return elevated_card(cx)
            .child(section_header(cx, "History", "no commits yet"))
            .into_any_element();
    }
    elevated_card(cx)
        .child(section_header(
            cx,
            "History",
            &format!("{} commits", history.len()),
        ))
        .child(
            div()
                .id("git-history-scroll")
                .mt(px(6.))
                .max_h(px(260.))
                .overflow_scroll()
                .border_1()
                .border_color(theme::border(cx))
                .rounded(px(8.))
                .children(history.iter().map(|c| {
                    let is_sel = selected == Some(c.full_hash.as_str());
                    let hash = c.full_hash.clone();
                    div()
                        .cursor_pointer()
                        .flex()
                        .flex_col()
                        .px(px(12.))
                        .py(px(6.))
                        .border_b_1()
                        .border_color(theme::border(cx))
                        .when(is_sel, |this| this.bg(theme::bg_hover(cx)))
                        .hover(|this| this.bg(theme::bg_hover(cx)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _ev, window, cx| {
                                this.select_commit(&hash, window, cx);
                            }),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme::fg(cx))
                                .child(c.subject.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .text_xs()
                                .text_color(theme::muted(cx))
                                .child(div().font_family("monospace").child(c.hash.clone()))
                                .child(c.author.clone())
                                .child(format!("· {}", c.relative_date))
                                .children(c.tags.iter().map(|t| {
                                    div()
                                        .px(px(6.))
                                        .rounded(px(4.))
                                        .bg(theme::accent_light(cx))
                                        .text_color(theme::accent(cx))
                                        .child(t.clone())
                                })),
                        )
                })),
        )
        .child(
            div().mt(px(10.)).child(match detail {
                Some(d) => render_commit_detail(d, cx).into_any_element(),
                None if selected.is_some() => div()
                    .text_sm()
                    .text_color(theme::muted(cx))
                    .child("Loading…")
                    .into_any_element(),
                None => div()
                    .text_sm()
                    .text_color(theme::fg_secondary(cx))
                    .child("Select a commit")
                    .into_any_element(),
            }),
        )
        .into_any_element()
}

fn render_commit_detail(detail: &CommitDetail, cx: &mut Context<GitPage>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .p(px(10.))
        .rounded(px(8.))
        .bg(theme::bg_secondary(cx))
        .border_1()
        .border_color(theme::border(cx))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(detail.entry.subject.clone()),
                )
                .child(
                    div()
                        .font_family("monospace")
                        .text_xs()
                        .text_color(theme::muted(cx))
                        .child(detail.entry.hash.clone()),
                ),
        )
        .children(detail.files.iter().map(|f| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .text_xs()
                .child(
                    div()
                        .font_family("monospace")
                        .text_color(theme::fg(cx))
                        .child(f.path.clone()),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        // Binary files honestly show no +/- count (T038: additions/deletions parse to None, never 0).
                        .child(
                            div()
                                .text_color(theme::accent(cx))
                                .child(match f.additions {
                                    Some(n) => format!("+{n}"),
                                    None => "binary".to_string(),
                                }),
                        )
                        .when_some(f.deletions, |el, n| {
                            el.child(div().text_color(theme::danger(cx)).child(format!("−{n}")))
                        }),
                )
        }))
}

/// Remotes view: remote cards + Add Remote form (T038).
fn remotes_view(
    remotes: &[RemoteEntry],
    remote_name_input: Entity<InputState>,
    remote_url_input: Entity<InputState>,
    busy: bool,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    elevated_card(cx)
        .child(section_header(
            cx,
            "Remotes",
            &format!("{} configured", remotes.len()),
        ))
        .when(remotes.is_empty(), |el| {
            el.child(
                div()
                    .mt(px(6.))
                    .text_sm()
                    .text_color(theme::fg_secondary(cx))
                    .child("No remotes configured"),
            )
        })
        .children(remotes.iter().map(|r| {
            let name = r.name.clone();
            let name_for_fetch = name.clone();
            div()
                .mt(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .p(px(10.))
                .rounded(px(8.))
                .border_1()
                .border_color(theme::border(cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_sm()
                                .font_weight(gpui::FontWeight::BOLD)
                                .child(r.name.clone())
                                .when(r.fetch_url.is_some(), |el| {
                                    el.child(
                                        div()
                                            .px(px(6.))
                                            .rounded(px(4.))
                                            .text_xs()
                                            .bg(theme::accent_light(cx))
                                            .text_color(theme::accent(cx))
                                            .child("fetch"),
                                    )
                                })
                                .when(r.push_url.is_some(), |el| {
                                    el.child(
                                        div()
                                            .px(px(6.))
                                            .rounded(px(4.))
                                            .text_xs()
                                            .bg(theme::accent_light(cx))
                                            .text_color(theme::accent(cx))
                                            .child("push"),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .font_family("monospace")
                                .text_xs()
                                .text_color(theme::muted(cx))
                                .child(r.fetch_url.clone().unwrap_or_default()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(render_action_button(
                            "Fetch",
                            busy,
                            cx,
                            move |this, _ev, window, cx| {
                                this.remote_fetch(&name_for_fetch, window, cx);
                            },
                        ))
                        .child(render_action_button(
                            "Delete",
                            busy,
                            cx,
                            move |this, _ev, window, cx| {
                                this.remote_delete(&name, window, cx);
                            },
                        )),
                )
        }))
        .child(
            div()
                .mt(px(10.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(div().flex_1().child(Input::new(&remote_name_input)))
                .child(div().flex_1().child(Input::new(&remote_url_input)))
                .child(render_action_button(
                    "Add remote",
                    busy,
                    cx,
                    |this, _ev, window, cx| {
                        this.add_remote_action(window, cx);
                    },
                )),
        )
}

// --- sub-renders -----------------------------------------------------------

fn render_header(
    status: Option<&RepoStatus>,
    no_repo: bool,
    error: Option<String>,
    refreshing: bool,
    busy: bool,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    elevated_card(cx)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(if no_repo || (status.is_none() && !refreshing) {
                    div()
                        .text_color(theme::fg_secondary(cx))
                        .text_sm()
                        .child("Not a git repository")
                } else if let Some(s) = status {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_sm()
                        .child(
                            div()
                                .text_color(theme::muted(cx))
                                .child(format!("{} ", s.workdir.display())),
                        )
                        .child(
                            div()
                                .text_color(theme::accent(cx))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child(format!("⏵ {}", s.branch)),
                        )
                } else {
                    div()
                        .text_color(theme::muted(cx))
                        .text_sm()
                        .child("Loading…")
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(refresh_button(refreshing, cx))
                        .child(pin_button(cx)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(render_action_button(
                            "Pull",
                            busy,
                            cx,
                            |this, _ev, window, cx| {
                                this.pull(window, cx);
                            },
                        ))
                        .child(render_action_button(
                            "Push",
                            busy,
                            cx,
                            |this, _ev, window, cx| {
                                this.push(window, cx);
                            },
                        )),
                ),
        )
        .when_some(error, |el, error| {
            el.child(div().text_sm().text_color(theme::danger(cx)).child(error))
        })
}

fn refresh_button(refreshing: bool, cx: &mut Context<GitPage>) -> impl IntoElement {
    div()
        .cursor_pointer()
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .bg(theme::bg_hover(cx))
        .text_sm()
        .text_color(theme::fg(cx))
        .when(refreshing, |this| this.opacity(0.5))
        .hover(|this| this.bg(theme::bg(cx)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _ev, _window, cx| {
                this.refresh(cx);
            }),
        )
        .child("↻ Refresh")
}

fn pin_button(cx: &mut Context<GitPage>) -> impl IntoElement {
    div()
        .cursor_pointer()
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .bg(theme::bg_hover(cx))
        .text_sm()
        .text_color(theme::fg(cx))
        .hover(|this| this.bg(theme::bg(cx)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _ev, _window, cx| {
                this.toggle_pin(cx);
            }),
        )
        .child("📌 Pin")
}

fn render_branches(
    branches: &[String],
    current: &str,
    branch_input: Entity<InputState>,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    let current = current.to_string();
    elevated_card(cx)
        .child(section_header(
            cx,
            "Branches",
            &format!("{} local", branches.len()),
        ))
        .child(
            div()
                .mt(px(6.))
                .flex()
                .flex_wrap()
                .gap(px(6.))
                .children(branches.iter().map(|name| {
                    let name = name.clone();
                    let is_current = name == current;
                    div()
                        .cursor_pointer()
                        .px(px(10.))
                        .py(px(4.))
                        .rounded(px(6.))
                        .text_sm()
                        .when(is_current, |this| {
                            this.bg(theme::accent(cx))
                                .text_color(theme::bg(cx))
                                .font_weight(gpui::FontWeight::BOLD)
                        })
                        .when(!is_current, |this| {
                            this.bg(theme::bg_hover(cx))
                                .text_color(theme::fg(cx))
                                .hover(|this| this.bg(theme::border(cx)))
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener({
                                let name = name.clone();
                                move |this, _ev, window, cx| {
                                    if name
                                        != this
                                            .status
                                            .as_ref()
                                            .map(|s| s.branch.as_str())
                                            .unwrap_or("")
                                    {
                                        this.checkout_branch(&name, window, cx);
                                    }
                                }
                            }),
                        )
                        .child(if is_current {
                            format!("● {name}")
                        } else {
                            name
                        })
                })),
        )
        .child(
            div()
                .mt(px(10.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(div().flex_1().child(Input::new(&branch_input)))
                .child(
                    div()
                        .cursor_pointer()
                        .px(px(12.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .text_sm()
                        .font_weight(gpui::FontWeight::BOLD)
                        .bg(theme::accent(cx))
                        .text_color(theme::bg(cx))
                        .hover(|this| this.bg(theme::accent_hover(cx)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _ev, window, cx| {
                                this.create_branch(window, cx);
                            }),
                        )
                        .child("Create & switch"),
                ),
        )
}

fn render_file_section(
    label: &'static str,
    entries: &[chronos_fm_services::git::RepoEntry],
    show_stage: bool,
    staged_section: bool,
    color: Hsla,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    if entries.is_empty() {
        return div().into_any_element();
    }
    let count = entries.len();
    // Staged section: show_stage=false (unstage button); diff uses staged=true.
    let diff_staged = !show_stage || staged_section;
    div()
        .child(section_header(
            cx,
            label,
            &format!("{count} file{}", if count == 1 { "" } else { "s" }),
        ))
        .child(
            div()
                .mt(px(4.))
                .border_1()
                .border_color(theme::border(cx))
                .rounded(px(8.))
                .overflow_x_hidden()
                .children(entries.iter().map(|entry| {
                    let path = entry.path.clone();
                    let path_for_diff = entry.path.clone();
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(12.))
                        .py(px(6.))
                        .hover(|this| this.bg(theme::bg_hover(cx)))
                        .child(
                            div()
                                .cursor_pointer()
                                .text_sm()
                                .text_color(color)
                                .child(entry.path.clone())
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener({
                                        let path = path_for_diff.clone();
                                        move |this, _ev, window, cx| {
                                            this.show_diff(&path, diff_staged, window, cx);
                                        }
                                    }),
                                ),
                        )
                        .child(
                            div().flex().gap(px(6.)).child(
                                div()
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .bg(theme::bg_hover(cx))
                                    .text_color(theme::fg(cx))
                                    .hover(|this| this.bg(theme::border(cx)))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener({
                                            let path = path.clone();
                                            move |this, _ev, window, cx| {
                                                if show_stage {
                                                    this.stage(&path, window, cx);
                                                } else {
                                                    this.unstage(&path, window, cx);
                                                }
                                            }
                                        }),
                                    )
                                    .child(if show_stage { "+ Stage" } else { "− Unstage" }),
                            ),
                        )
                })),
        )
        .into_any_element()
}

fn render_diff_panel(diff: &DiffSelection, cx: &mut Context<GitPage>) -> impl IntoElement {
    let title = if diff.staged {
        format!("Diff (staged) — {}", diff.path)
    } else {
        format!("Diff (worktree) — {}", diff.path)
    };
    elevated_card(cx)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(section_header(cx, &title, "unified"))
                .child(
                    div()
                        .cursor_pointer()
                        .text_xs()
                        .text_color(theme::muted(cx))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _ev, _w, cx| {
                                this.selected_diff = None;
                                cx.notify();
                            }),
                        )
                        .child("Close"),
                ),
        )
        .child(
            div()
                .id("git-diff-scroll")
                .mt(px(6.))
                .max_h(px(280.))
                .overflow_scroll()
                .p(px(10.))
                .rounded(px(8.))
                .bg(theme::bg_secondary(cx))
                .border_1()
                .border_color(theme::border(cx))
                .font_family("monospace")
                .text_xs()
                .text_color(theme::fg(cx))
                .child(diff.text.clone()),
        )
}

fn render_commit_bar(
    input: Entity<InputState>,
    staged_empty: bool,
    amend: bool,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    // T038 §3: Amend can commit even with nothing staged (message-only
    // amend); a plain commit still requires staged changes.
    let can_commit = amend || !staged_empty;
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(12.))
        .bg(theme::bg_secondary(cx))
        .border_1()
        .border_color(theme::border(cx))
        .rounded(px(12.))
        .shadow_md()
        .child(
            // Commit/Amend segmented control (mockup §Changes toolbar).
            div()
                .flex()
                .gap(px(4.))
                .p(px(2.))
                .rounded(px(6.))
                .bg(theme::bg(cx))
                .child(commit_mode_option("Commit", !amend, false, cx))
                .child(commit_mode_option("Amend", amend, true, cx)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(div().flex_1().child(Input::new(&input)))
                .child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .text_sm()
                        .font_weight(gpui::FontWeight::BOLD)
                        .when(!can_commit, |this| {
                            this.bg(theme::border(cx))
                                .text_color(theme::muted(cx))
                                .opacity(0.6)
                        })
                        .when(can_commit, |this| {
                            this.bg(theme::accent(cx))
                                .text_color(theme::bg(cx))
                                .hover(|this| this.bg(theme::accent_hover(cx)))
                                .cursor_pointer()
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _ev, window, cx| {
                                if can_commit {
                                    this.commit(window, cx);
                                }
                            }),
                        )
                        .child(if amend { "Amend" } else { "Commit" }),
                ),
        )
}

fn commit_mode_option(
    label: &'static str,
    active: bool,
    target_amend: bool,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    div()
        .cursor_pointer()
        .px(px(10.))
        .py(px(4.))
        .rounded(px(5.))
        .text_xs()
        .when(active, |this| {
            this.bg(theme::accent(cx))
                .text_color(theme::bg(cx))
                .font_weight(gpui::FontWeight::BOLD)
        })
        .when(!active, |this| this.text_color(theme::fg_secondary(cx)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _ev, _window, cx| {
                if this.amend != target_amend {
                    this.toggle_amend(cx);
                }
            }),
        )
        .child(label)
}

fn render_stash_section(
    entries: &[StashEntry],
    stash_input: Entity<InputState>,
    busy: bool,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    elevated_card(cx)
        .child(section_header(
            cx,
            "Stash",
            &format!(
                "{} entr{}",
                entries.len(),
                if entries.len() == 1 { "y" } else { "ies" }
            ),
        ))
        .children(entries.iter().map(|entry| {
            let index = entry.index;
            let desc = format!(
                "stash@{{{}}}: {} -- {}",
                entry.index, entry.branch, entry.message
            );
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(12.))
                .py(px(4.))
                .text_sm()
                .text_color(theme::fg(cx))
                .child(div().truncate().child(desc))
                .child(
                    div()
                        .flex()
                        .gap(px(4.))
                        .child(render_action_button(
                            "Pop",
                            busy,
                            cx,
                            move |this, _ev, window, cx| {
                                this.stash_pop(index, window, cx);
                            },
                        ))
                        .child(render_action_button(
                            "Drop",
                            busy,
                            cx,
                            move |this, _ev, window, cx| {
                                this.stash_drop(index, window, cx);
                            },
                        )),
                )
        }))
        .child(
            div()
                .mt(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(div().flex_1().child(Input::new(&stash_input)))
                .child(render_action_button(
                    "Stash push",
                    busy,
                    cx,
                    |this, _ev, window, cx| {
                        this.stash_push_action(window, cx);
                    },
                )),
        )
}

fn render_action_button(
    label: &'static str,
    busy: bool,
    cx: &mut Context<GitPage>,
    f: impl Fn(&mut GitPage, &MouseDownEvent, &mut Window, &mut Context<GitPage>) + 'static,
) -> impl IntoElement {
    div()
        .px(px(8.))
        .py(px(2.))
        .rounded(px(4.))
        .text_xs()
        .font_weight(gpui::FontWeight::MEDIUM)
        .when(busy, |this| {
            this.bg(theme::border(cx))
                .text_color(theme::muted(cx))
                .opacity(0.6)
        })
        .when(!busy, |this| {
            this.bg(theme::bg_hover(cx))
                .text_color(theme::fg(cx))
                .hover(|this| this.bg(theme::border(cx)))
                .cursor_pointer()
        })
        .on_mouse_down(MouseButton::Left, cx.listener(f))
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::{resolve_follow_dir, should_refresh};
    use std::path::PathBuf;

    #[test]
    fn pinned_path_takes_precedence_over_explorer_path() {
        let pinned = PathBuf::from("/repo/pinned");
        assert_eq!(
            resolve_follow_dir(Some(&pinned), Some("/explorer/cwd")),
            Some(pinned)
        );
    }

    #[test]
    fn follow_mode_uses_active_explorer_directory() {
        assert_eq!(
            resolve_follow_dir(None, Some("/explorer/cwd")),
            Some(PathBuf::from("/explorer/cwd"))
        );
    }

    #[test]
    fn follow_mode_without_explorer_resolves_to_none() {
        assert_eq!(resolve_follow_dir(None, None), None);
    }

    #[test]
    fn refresh_loop_triggers_on_watcher_signal() {
        assert!(should_refresh(
            true,
            Some(&PathBuf::from("/a")),
            Some(&PathBuf::from("/a"))
        ));
    }

    #[test]
    fn refresh_loop_triggers_on_directory_change() {
        assert!(should_refresh(
            false,
            Some(&PathBuf::from("/b")),
            Some(&PathBuf::from("/a"))
        ));
    }

    #[test]
    fn refresh_loop_idles_without_signal_or_navigation() {
        assert!(!should_refresh(
            false,
            Some(&PathBuf::from("/a")),
            Some(&PathBuf::from("/a"))
        ));
        // First tick after construction: no last dir yet → refresh.
        assert!(should_refresh(false, Some(&PathBuf::from("/a")), None));
    }
}
