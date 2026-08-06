//! Live Git status panel (T010, Milestone A).
//!
//! Three sections (Staged / Modified / Untracked) with per-row stage/unstage
//! actions, a commit bar, and a directory mode (follow the active explorer
//! tab or pin to a specific path).
//!
//! Repository status is computed on the background executor and refreshed by a
//! `.git`-directory watcher (400 ms debounce), following the same channel +
//! foreground-poll pattern as `config.toml` hot reload in `root.rs`.

use crate::explorer::ExplorerPage;
use chronos_fm_services::git::{self, GitError, RepoStatus};
use chronos_fm_services::git::watcher::GitWatcher;
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::*;
use gpui::prelude::*;
use gpui_component::input::Input;
use gpui_component::input::InputState;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

/// The live Git status panel.
pub struct GitPage {
    explorer: WeakEntity<ExplorerPage>,
    pinned_path: Option<PathBuf>,
    status: Option<RepoStatus>,
    no_repo: bool,
    error: Option<String>,
    refreshing: bool,
    message_input: Entity<InputState>,
    _watcher: Option<GitWatcher>,
    _shutdown_tx: Option<mpsc::Sender<()>>,
    refresh_tx: Option<mpsc::Sender<()>>,
}

impl GitPage {
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
            _watcher: None,
            _shutdown_tx: Some(tx),
            refresh_tx: Some(refresh_tx),
        };
        page.start_refresh_loop(window, rx, cx);
        page
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        self.do_refresh(dir, cx);
    }

    fn current_dir(&self, cx: &mut Context<Self>) -> Option<PathBuf> {
        if let Some(p) = &self.pinned_path {
            return Some(p.clone());
        }
        let explorer = self.explorer.upgrade()?;
        Some(PathBuf::from(explorer.read(cx).current_path(cx)))
    }

    fn do_refresh(&mut self, dir: Option<PathBuf>, cx: &mut Context<Self>) {
        let Some(dir) = dir else { return };
        self.refreshing = true;
        cx.notify();

        let this = cx.weak_entity();
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = cx
                    .background_executor()
                    .spawn(async move { git::open_repo(&dir).and_then(|repo| git::status(&repo)) })
                    .await;
                this.update(&mut cx, |page, cx| {
                    page.refreshing = false;
                    match result {
                        Ok(status) => {
                            page.recreate_watcher(&status.git_dir);
                            page.status = Some(status);
                            page.no_repo = false;
                            page.error = None;
                        }
                        Err(GitError::NotARepository) => {
                            page.status = None;
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
                        if !changed {
                            continue;
                        }
                        if this
                            .update_in(&mut cx, |page, _w, cx| page.refresh(cx))
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
        let this = cx.weak_entity();
        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let result = cx.background_executor()
                    .spawn(async move {
                        let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                        let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                        git::stage_path(&repo, &path).map_err(|e| e.to_string())
                    }).await;
                this.update_in(&mut cx, |page, _window, cx| {
                    if let Err(msg) = result {
                        page.error = Some(format!("stage {err_path}: {msg}"));
                    }
                    page.refresh(cx);
                }).ok();
            }
        }).detach();
    }

    fn unstage(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        let path = path.to_string();
        let err_path = path.clone();
        let this = cx.weak_entity();
        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let result = cx.background_executor()
                    .spawn(async move {
                        let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                        let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                        git::unstage_path(&repo, &path).map_err(|e| e.to_string())
                    }).await;
                this.update_in(&mut cx, |page, _window, cx| {
                    if let Err(msg) = result {
                        page.error = Some(format!("unstage {err_path}: {msg}"));
                    }
                    page.refresh(cx);
                }).ok();
            }
        }).detach();
    }

    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dir = self.current_dir(cx);
        let message = self.message_input.read(cx).text().to_string();
        if message.trim().is_empty() {
            self.error = Some("commit message is empty".to_string());
            cx.notify();
            return;
        }
        let this = cx.weak_entity();
        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let result = cx.background_executor()
                    .spawn(async move {
                        let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                        let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                        git::commit(&repo, &message).map_err(|e| e.to_string())
                    }).await;
                this.update_in(&mut cx, |page, _window, cx| {
                    if let Err(msg) = result {
                        page.error = Some(format!("commit: {msg}"));
                    }
                    page.refresh(cx);
                }).ok();
            }
        }).detach();
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
    fn render_body(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.status.clone();
        let no_repo = self.no_repo;
        let error = self.error.clone();
        let refreshing = self.refreshing;

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(px(24.))
            .bg(theme::bg(cx))
            .child(render_header(status.as_ref(), no_repo, error, refreshing, cx))
            .when(status.is_some(), |el| {
                let s = status.as_ref().unwrap();
                el.child(render_file_section("Staged", &s.staged, false, theme::accent(cx), cx))
                    .child(render_file_section("Modified", &s.modified, true, theme::muted(cx), cx))
                    .child(render_file_section("Untracked", &s.untracked, true, theme::fg_secondary(cx), cx))
                    .child(render_commit_bar(self.message_input.clone(), s.staged.is_empty(), cx))
            })
    }
}

// --- sub-renders -----------------------------------------------------------

fn render_header(
    status: Option<&RepoStatus>,
    no_repo: bool,
    error: Option<String>,
    refreshing: bool,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    elevated_card(cx)
        .child(
            div().flex().items_center().justify_between()
                .child(if no_repo || (status.is_none() && !refreshing) {
                    div().text_color(theme::fg_secondary(cx)).text_sm().child("Not a git repository")
                } else if let Some(s) = status {
                    div().flex().items_center().gap(px(8.)).text_sm()
                        .child(div().text_color(theme::muted(cx)).child(format!("{} ", s.workdir.display())))
                        .child(div().text_color(theme::accent(cx)).font_weight(gpui::FontWeight::BOLD).child(format!("⏵ {}", s.branch)))
                } else {
                    div().text_color(theme::muted(cx)).text_sm().child("Loading…")
                })
                .child(
                    div().flex().items_center().gap(px(8.))
                        .child(refresh_button(refreshing, cx))
                        .child(pin_button(cx)),
                ),
        )
        .when(error.is_some(), |el| {
            el.child(div().text_sm().text_color(theme::danger(cx)).child(error.unwrap()))
        })
}

fn refresh_button(refreshing: bool, cx: &mut Context<GitPage>) -> impl IntoElement {
    div()
        .cursor_pointer()
        .px(px(10.)).py(px(4.)).rounded(px(6.))
        .bg(theme::bg_hover(cx))
        .text_sm().text_color(theme::fg(cx))
        .when(refreshing, |this| this.opacity(0.5))
        .hover(|this| this.bg(theme::bg(cx)))
        .on_mouse_down(MouseButton::Left, cx.listener(|this, _ev, _window, cx| {
            this.refresh(cx);
        }))
        .child("↻ Refresh")
}

fn pin_button(cx: &mut Context<GitPage>) -> impl IntoElement {
    div()
        .cursor_pointer()
        .px(px(10.)).py(px(4.)).rounded(px(6.))
        .bg(theme::bg_hover(cx))
        .text_sm().text_color(theme::fg(cx))
        .hover(|this| this.bg(theme::bg(cx)))
        .on_mouse_down(MouseButton::Left, cx.listener(|this, _ev, _window, cx| {
            this.toggle_pin(cx);
        }))
        .child("📌 Pin")
}

fn render_file_section(
    label: &str,
    entries: &[chronos_fm_services::git::RepoEntry],
    show_stage: bool,
    color: Hsla,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    if entries.is_empty() {
        return div().into_any_element();
    }
    let count = entries.len();
    div()
        .child(section_header(cx, label, &format!("{count} file{}", if count == 1 { "" } else { "s" })))
        .child(
            div().mt(px(4.)).border_1().border_color(theme::border(cx)).rounded(px(8.)).overflow_x_hidden()
                .children(entries.iter().map(|entry| {
                    let path = entry.path.clone();
                    div()
                        .flex().items_center().justify_between()
                        .px(px(12.)).py(px(6.))
                        .hover(|this| this.bg(theme::bg_hover(cx)))
                        .child(div().text_sm().text_color(color).child(entry.path.clone()))
                        .child(
                            div().flex().gap(px(6.)).child(
                                div()
                                    .cursor_pointer()
                                    .px(px(8.)).py(px(2.)).rounded(px(4.))
                                    .text_xs().font_weight(gpui::FontWeight::MEDIUM)
                                    .bg(theme::bg_hover(cx)).text_color(theme::fg(cx))
                                    .hover(|this| this.bg(theme::border(cx)))
                                    .on_mouse_down(MouseButton::Left, cx.listener({
                                        let path = path.clone();
                                        move |this, _ev, window, cx| {
                                            if show_stage {
                                                this.stage(&path, window, cx);
                                            } else {
                                                this.unstage(&path, window, cx);
                                            }
                                        }
                                    }))
                                    .child(if show_stage { "+ Stage" } else { "− Unstage" }),
                            ),
                        )
                })),
        )
        .into_any_element()
}

fn render_commit_bar(
    input: Entity<InputState>,
    staged_empty: bool,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    div()
        .flex().items_center().gap(px(8.))
        .p(px(12.))
        .bg(theme::bg_secondary(cx))
        .border_1().border_color(theme::border(cx))
        .rounded(px(12.)).shadow_md()
        .child(div().flex_1().child(Input::new(&input)))
        .child(
            div()
                .px(px(12.)).py(px(6.)).rounded(px(6.))
                .text_sm().font_weight(gpui::FontWeight::BOLD)
                .when(staged_empty, |this| {
                    this.bg(theme::border(cx)).text_color(theme::muted(cx)).opacity(0.6)
                })
                .when(!staged_empty, |this| {
                    this.bg(theme::accent(cx)).text_color(theme::bg(cx))
                        .hover(|this| this.bg(theme::accent_hover(cx)))
                        .cursor_pointer()
                })
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _ev, window, cx| {
                    if !staged_empty {
                        this.commit(window, cx);
                    }
                }))
                .child("Commit"),
        )
}

// render test deferred to T010 verification step (gpui::test recursion_limit)
#[cfg(test)]
mod tests {
    // #[gpui::test]
    // async fn git_page_renders_without_panicking(...) — see TODO above
}
