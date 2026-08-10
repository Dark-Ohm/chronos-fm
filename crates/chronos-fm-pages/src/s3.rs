//! S3 page — object-storage browser with interactive credentials flow
//! and embedded ExplorerPane for bucket/prefix navigation (T011).

use std::sync::Arc;

use chronos_fm_core::config::Config;
use chronos_fm_core::config::s3_credentials::S3CredentialsManager;
use chronos_fm_models::file_entry::FileEntryDto;
use chronos_fm_services::fs::provider::FileSystemProvider;
use chronos_fm_services::s3::transfer::{
    CancelHandle, TransferDirection, TransferEvent, TransferJob, TransferState,
};
use chronos_fm_services::s3::{self, S3Client};
use chronos_fm_ui::patterns::elevated_card;
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::Disableable;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};

use crate::explorer::ExplorerPane;

gpui::actions!(s3, [NavigateToSettings]);

/// Which phase the S3 page is in.
enum S3State {
    NoProfiles,
    NeedCredentials,
    Connecting,
    Browsing,
    Error { message: String },
}

/// Which of the four T039 sub-views is active (design spec §4, mirrors
/// T038's `GitView` pattern — a horizontal tab strip rather than the
/// mockup's left-column sidebar nav, same documented deviation).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum S3View {
    /// The existing embedded `ExplorerPane` — unchanged (T011/T021).
    #[default]
    Explorer,
    /// `list_buckets()` result as its own view.
    Buckets,
    /// Chunked upload/download job queue (T039 §1/§2).
    Transfers,
    /// Selected-object metadata; honest empty state when nothing selected.
    Properties,
}

impl S3View {
    fn label(self) -> &'static str {
        match self {
            S3View::Explorer => "Explorer",
            S3View::Buckets => "Buckets",
            S3View::Transfers => "Transfers",
            S3View::Properties => "Properties",
        }
    }

    /// Parses a sub-view name from `--page=s3:<name>` (T046 residual,
    /// case-insensitive). Mirrors `PageKind::from_cli_name`.
    fn from_cli_name(name: &str) -> Option<S3View> {
        match name.trim().to_lowercase().as_str() {
            "explorer" => Some(S3View::Explorer),
            "buckets" => Some(S3View::Buckets),
            "transfers" => Some(S3View::Transfers),
            "properties" => Some(S3View::Properties),
            _ => None,
        }
    }
}

/// One row in the Transfers view: the job's current state plus the
/// `CancelHandle` the Cancel button calls into (design spec §1 —
/// cooperative cancellation, checked between chunks).
struct TransferRow {
    job: TransferJob,
    cancel: CancelHandle,
}

pub struct S3Page {
    config: Config,
    focus_handle: FocusHandle,
    access_key_input: Entity<InputState>,
    secret_key_input: Entity<InputState>,
    state: S3State,
    /// The embedded explorer pane, created when connecting and repurposed
    /// for browsing. None until the first connect attempt.
    s3_pane: Option<Entity<ExplorerPane>>,
    /// Which sub-view is active. Only meaningful once `state` is
    /// `Browsing`; `content()` still routes on `state` first.
    view: S3View,
    /// The typed client, kept alongside the type-erased
    /// `FileSystemProvider` wired into `s3_pane` so Buckets/Transfers can
    /// call `list_buckets`/`spawn_upload`/`spawn_download` directly.
    client: Option<Arc<S3Client>>,
    buckets: Vec<FileEntryDto>,
    buckets_loading: bool,
    transfers: Vec<TransferRow>,
    next_job_id: u64,
    /// Last Buckets/Transfers-view error. Separate from `S3State::Error`
    /// (the connect flow) so a failed bucket list or a picker error
    /// doesn't kick the page back to the credentials form.
    view_error: Option<String>,
}

impl Focusable for S3Page {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl S3Page {
    pub fn new(config: Config, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let access_key_input = cx.new(|cx| InputState::new(window, cx));
        let secret_key_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_masked(true, window, cx);
            state
        });
        let state = Self::derive_state(&config);
        Self {
            config,
            focus_handle: cx.focus_handle(),
            access_key_input,
            secret_key_input,
            state,
            s3_pane: None,
            view: S3View::default(),
            client: None,
            buckets: Vec::new(),
            buckets_loading: false,
            transfers: Vec::new(),
            next_job_id: 0,
            view_error: None,
        }
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
        if matches!(self.state, S3State::Connecting) {
            return;
        }
        if matches!(self.state, S3State::Browsing) {
            // Only reset if profile changed.
            return;
        }
        self.state = Self::derive_state(&self.config);
    }

    fn derive_state(config: &Config) -> S3State {
        if config.s3.profiles.is_empty() || config.s3.default_profile.is_empty() {
            S3State::NoProfiles
        } else {
            S3State::NeedCredentials
        }
    }

    fn access_key_text(&self, cx: &App) -> String {
        self.access_key_input.read(cx).text().to_string()
    }

    fn secret_key_text(&self, cx: &App) -> String {
        self.secret_key_input.read(cx).text().to_string()
    }

    /// Validate inputs, create an embedded ExplorerPane, and spawn the
    /// async connect flow. The pane is created synchronously (before the
    /// spawn) so it exists for the Connecting render; the S3 provider is
    /// wired in asynchronously when the client is ready.
    fn start_connect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let profile_name = self.config.s3.default_profile.clone();
        let profile = match self.config.s3.profiles.get(&profile_name) {
            Some(p) => chronos_fm_services::s3::S3Profile {
                endpoint: p.endpoint.clone(),
                region: p.region.clone(),
                force_path_style: p.force_path_style,
            },
            None => {
                self.state = S3State::Error {
                    message: format!("Profile '{profile_name}' not found"),
                };
                cx.notify();
                return;
            }
        };
        let access_key = self.access_key_text(cx);
        let secret_key = self.secret_key_text(cx);

        if access_key.is_empty() || secret_key.is_empty() {
            self.state = S3State::Error {
                message: "Access key and secret key are required".to_string(),
            };
            cx.notify();
            return;
        }

        // Create the pane now so the Connecting render has somewhere to go.
        // Mark it loaded so it doesn't try to reload the local cwd.
        let pane = cx.new(|cx| {
            let mut p = ExplorerPane::build(None, window, cx);
            p.loaded = true;
            p
        });
        self.s3_pane = Some(pane.clone());
        self.state = S3State::Connecting;
        cx.notify();

        let s3_root = s3::s3_profile_root(&profile_name);

        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                if let Ok(Some(manager)) = S3CredentialsManager::connect().await {
                    let _ = manager
                        .store(&profile_name, &access_key, &secret_key)
                        .await;
                }

                match S3Client::from_profile(
                    &profile_name,
                    &profile,
                    &access_key,
                    &secret_key,
                ) {
                    Ok(client) => {
                        let client = Arc::new(client);
                        if let Err(error) = this.update_in(&mut cx, |this, window, cx| {
                            // Wire the S3 provider into the pane, point it at
                            // the profile root (bucket listing), and kick off
                            // the async listing **immediately**. `reload()` is
                            // a no-op for provider-backed panes (B.1), so
                            // deferring to the render path via `loaded = false`
                            // would show an empty list forever (T021).
                            //
                            // The pane is updated through plain `Entity::update`
                            // on this S3Page context, reusing the `window` this
                            // callback already holds. A `WeakEntity::update_in`
                            // here would fail with "entity has no current
                            // window": the pane is created with `cx.new` and
                            // never goes through `defer_in`/`observe_in`/
                            // `subscribe_in` — the only paths that register an
                            // entity in `current_window_by_entity` (regression:
                            // commit `32cb1ac`).
                            this.client = Some(client.clone());
                            this.wire_pane(window, &pane, client, s3_root, cx);
                        }) {
                            tracing::error!(
                                "S3 connect: page released before the client connected: {error}"
                            );
                        }
                    }
                    Err(error) => {
                        tracing::error!(
                            "S3 connect failed for profile '{profile_name}': {error}"
                        );
                        let _ = this.update_in(&mut cx, |this, _window, cx| {
                            this.state = S3State::Error {
                                message: format!("Failed to connect: {error}"),
                            };
                            cx.notify();
                        });
                    }
                }
            }
        })
        .detach();
    }

    /// Wire an S3 provider into the embedded pane, point it at `s3_root` (the
    /// profile root — bucket listing), and kick off the initial async listing.
    ///
    /// Extracted from the connect callback so the S3Page → pane handoff is
    /// directly testable (T021). The caller passes the `window` it already
    /// holds from `spawn_in` / `update_in`; the pane is updated through plain
    /// [`Entity::update`] on this S3Page context, never a
    /// `WeakEntity::update_in` — a pane created with `cx.new` is not registered
    /// in the window-by-entity map, so that would fail with "entity has no
    /// current window" and the bucket listing would never be requested
    /// (regression: commit `32cb1ac`).
    fn wire_pane(
        &mut self,
        window: &mut Window,
        pane: &Entity<ExplorerPane>,
        provider: Arc<dyn FileSystemProvider>,
        s3_root: String,
        cx: &mut Context<Self>,
    ) {
        pane.update(cx, |pane, cx| {
            pane.set_provider(provider);
            pane.cwd = s3_root;
            pane.reload_provider(window, cx);
        });
        self.state = S3State::Browsing;
        cx.notify();
    }

    // ---- T039: sub-views, chunked transfers ----

    fn select_view(&mut self, view: S3View, cx: &mut Context<Self>) {
        self.view = view;
        if view == S3View::Buckets && self.buckets.is_empty() && !self.buckets_loading {
            self.load_buckets(cx);
        }
        cx.notify();
    }

    /// `--page=s3:<name>` (T046 residual): select a sub-view by CLI name at
    /// startup. Silently warns and leaves the default view on an
    /// unrecognized name — never aborts the launch over a typo. A no-op
    /// while still in the credentials/connecting flow (nothing to route to
    /// yet) — `select_view` itself doesn't gate on `state`, but calling it
    /// before `Browsing` is harmless since `content()` routes on `state`
    /// first and only reaches `browsing_content` once connected.
    pub(crate) fn set_initial_subview(&mut self, name: &str, cx: &mut Context<Self>) {
        match S3View::from_cli_name(name) {
            Some(view) => self.select_view(view, cx),
            None => tracing::warn!("ignoring unrecognized s3 sub-view {name:?}"),
        }
    }

    /// The bucket + key-prefix the Explorer pane is currently pointed at,
    /// or `None` at the profile root (no bucket selected) — uploads need a
    /// destination bucket, so this gates the Upload button.
    fn current_bucket_prefix(&self, cx: &App) -> Option<(String, String)> {
        let cwd = self.s3_pane.as_ref()?.read(cx).cwd.clone();
        let (_, bucket, key) = s3::parse_s3_path(&cwd)?;
        if bucket.is_empty() {
            None
        } else {
            Some((bucket, key))
        }
    }

    /// The first selected file entry in the Explorer pane, if any —
    /// download source for Transfers and the subject of Properties.
    fn current_selection(&self, cx: &App) -> Option<FileEntryDto> {
        self.s3_pane
            .as_ref()?
            .read(cx)
            .filtered_entries_for_selection()
            .into_iter()
            .find(|e| e.kind == "file")
    }

    fn load_buckets(&mut self, cx: &mut Context<Self>) {
        let Some(client) = self.client.clone() else {
            return;
        };
        self.buckets_loading = true;
        cx.notify();
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = cx
                    .background_executor()
                    .spawn(async move { client.list_buckets().map_err(|e| e.to_string()) })
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.buckets_loading = false;
                    match result {
                        Ok(list) => {
                            this.buckets = list;
                            this.view_error = None;
                        }
                        Err(e) => this.view_error = Some(format!("list_buckets: {e}")),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Point the embedded pane at `bucket`'s root and switch to Explorer —
    /// the Buckets row's click action (mockup's "open-bucket").
    fn open_bucket(&mut self, bucket: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(pane), Some(client)) = (self.s3_pane.clone(), self.client.clone()) else {
            return;
        };
        let path = s3::s3_bucket_path(client.profile_name(), bucket);
        pane.update(cx, |pane, cx| {
            pane.cwd = path;
            pane.reload_provider(window, cx);
        });
        self.view = S3View::Explorer;
        cx.notify();
    }

    /// Apply one `TransferEvent` from a background transfer's channel to
    /// the matching row. No-op if the row was somehow removed (e.g. a
    /// future "clear finished" while a stale event is still in flight —
    /// defensive, not expected in practice since only terminal states are
    /// ever cleared).
    fn apply_transfer_event(&mut self, job_id: u64, event: TransferEvent) {
        let Some(row) = self.transfers.iter_mut().find(|r| r.job.id == job_id) else {
            return;
        };
        match event {
            TransferEvent::Progress(p) => {
                row.job.bytes_done = p.bytes_done;
                row.job.bytes_total = p.bytes_total;
            }
            TransferEvent::Completed { .. } => {
                let _ = row.job.transition_to(TransferState::Completed);
            }
            TransferEvent::Failed { reason, .. } => {
                let _ = row.job.transition_to(TransferState::Failed { reason });
            }
            TransferEvent::Cancelled { .. } => {
                let _ = row.job.transition_to(TransferState::Cancelled);
            }
        }
    }

    /// Upload flow: pick a local file, then start a chunked multipart
    /// upload into the Explorer pane's current bucket/prefix (design spec
    /// §1/§4). Requires a bucket to be selected first — see
    /// `current_bucket_prefix`.
    fn start_upload(&mut self, cx: &mut Context<Self>) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let Some((bucket, prefix)) = self.current_bucket_prefix(cx) else {
            self.view_error = Some("select a bucket in Explorer before uploading".to_string());
            cx.notify();
            return;
        };
        let job_id = self.next_job_id;
        self.next_job_id += 1;

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let picked = chronos_fm_services::dialogs::pick_file("Upload to S3").await;
                let local_path = match picked {
                    Ok(Some(p)) => p,
                    // Cancelled — no default path is ever silently
                    // substituted (design spec §5).
                    Ok(None) => return,
                    Err(e) => {
                        let _ = this.update(&mut cx, |this, cx| {
                            this.view_error = Some(format!("file picker: {e}"));
                            cx.notify();
                        });
                        return;
                    }
                };
                let file_name = local_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "upload".to_string());
                let key = format!("{prefix}{file_name}");
                let bytes_total = std::fs::metadata(&local_path).map(|m| m.len()).unwrap_or(0);

                let (cancel, rx) =
                    client.spawn_upload(job_id, local_path.clone(), bucket.clone(), key.clone());

                let started = this.update(&mut cx, |this, cx| {
                    this.view_error = None;
                    this.transfers.insert(
                        0,
                        TransferRow {
                            job: TransferJob {
                                id: job_id,
                                direction: TransferDirection::Upload,
                                local_path,
                                bucket,
                                key,
                                state: TransferState::InProgress,
                                bytes_done: 0,
                                bytes_total,
                            },
                            cancel,
                        },
                    );
                    cx.notify();
                });
                if started.is_err() {
                    return;
                }

                while let Ok(event) = rx.recv().await {
                    let terminal = matches!(
                        event,
                        TransferEvent::Completed { .. }
                            | TransferEvent::Failed { .. }
                            | TransferEvent::Cancelled { .. }
                    );
                    let updated = this.update(&mut cx, |this, cx| {
                        this.apply_transfer_event(job_id, event);
                        cx.notify();
                    });
                    if updated.is_err() || terminal {
                        break;
                    }
                }
            }
        })
        .detach();
    }

    /// Download flow: the currently selected Explorer object, saved under
    /// its own basename into a picked local directory.
    fn start_download(&mut self, cx: &mut Context<Self>) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let Some(entry) = self.current_selection(cx) else {
            self.view_error = Some("select an object in Explorer before downloading".to_string());
            cx.notify();
            return;
        };
        let Some((_, bucket, key)) = s3::parse_s3_path(&entry.path) else {
            self.view_error = Some(format!("not an S3 path: {}", entry.path));
            cx.notify();
            return;
        };
        let job_id = self.next_job_id;
        self.next_job_id += 1;
        let bytes_total = entry.size;
        let name = entry.name.clone();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let picked = chronos_fm_services::dialogs::pick_directory("Download to").await;
                let dest_dir = match picked {
                    Ok(Some(p)) => p,
                    Ok(None) => return,
                    Err(e) => {
                        let _ = this.update(&mut cx, |this, cx| {
                            this.view_error = Some(format!("directory picker: {e}"));
                            cx.notify();
                        });
                        return;
                    }
                };
                let local_path = dest_dir.join(&name);

                let (cancel, rx) =
                    client.spawn_download(job_id, bucket.clone(), key.clone(), local_path.clone());

                let started = this.update(&mut cx, |this, cx| {
                    this.view_error = None;
                    this.transfers.insert(
                        0,
                        TransferRow {
                            job: TransferJob {
                                id: job_id,
                                direction: TransferDirection::Download,
                                local_path,
                                bucket,
                                key,
                                state: TransferState::InProgress,
                                bytes_done: 0,
                                bytes_total,
                            },
                            cancel,
                        },
                    );
                    cx.notify();
                });
                if started.is_err() {
                    return;
                }

                while let Ok(event) = rx.recv().await {
                    let terminal = matches!(
                        event,
                        TransferEvent::Completed { .. }
                            | TransferEvent::Failed { .. }
                            | TransferEvent::Cancelled { .. }
                    );
                    let updated = this.update(&mut cx, |this, cx| {
                        this.apply_transfer_event(job_id, event);
                        cx.notify();
                    });
                    if updated.is_err() || terminal {
                        break;
                    }
                }
            }
        })
        .detach();
    }

    fn cancel_transfer(&mut self, job_id: u64, cx: &mut Context<Self>) {
        if let Some(row) = self.transfers.iter().find(|r| r.job.id == job_id) {
            row.cancel.cancel();
        }
        cx.notify();
    }

    fn clear_finished_transfers(&mut self, cx: &mut Context<Self>) {
        self.transfers.retain(|r| !r.job.state.is_terminal());
        cx.notify();
    }
}

impl Render for S3Page {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::bg(cx))
            .track_focus(&self.focus_handle)
            .child(header(cx))
            .child(content(self, cx))
    }
}

fn header(cx: &App) -> impl IntoElement {
    div()
        .px(px(16.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(theme::border(cx))
        .text_lg()
        .font_weight(FontWeight::BOLD)
        .text_color(theme::fg(cx))
        .child("☁️ S3")
}

fn content(page: &mut S3Page, cx: &mut Context<S3Page>) -> AnyElement {
    match &page.state {
        S3State::Connecting => {
            if let Some(pane) = &page.s3_pane {
                return div()
                    .flex_1()
                    .relative()
                    .child(pane.clone())
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .bg(gpui::Hsla::from(gpui::rgba(0x00000044)))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_color(theme::fg(cx))
                                    .text_lg()
                                    .child("Connecting…"),
                            ),
                    )
                    .into_any_element();
            }
            // Fallthrough: show connecting card if pane not created yet.
            connecting_card(cx)
        }
        S3State::Browsing => browsing_content(page, cx),
        _ => {
            let inner: AnyElement = match &page.state {
                S3State::NoProfiles => no_profiles_card(cx),
                S3State::NeedCredentials => credentials_form(page, cx),
                S3State::Error { message } => error_card(message.clone(), cx),
                _ => unreachable!(),
            };
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(inner)
                .into_any_element()
        }
    }
}

/// Sub-nav + routed view content, once connected (T039 §4).
fn browsing_content(page: &mut S3Page, cx: &mut Context<S3Page>) -> AnyElement {
    let Some(pane) = page.s3_pane.clone() else {
        return connecting_card(cx);
    };
    let view = page.view;
    let buckets_count = page.buckets.len();
    let active_transfers = page
        .transfers
        .iter()
        .filter(|r| !r.job.state.is_terminal())
        .count();

    let body: AnyElement = match view {
        S3View::Explorer => div().flex_1().relative().child(pane).into_any_element(),
        S3View::Buckets => buckets_view(page, cx),
        S3View::Transfers => transfers_view(page, cx),
        S3View::Properties => properties_view(page, cx),
    };

    div()
        .flex_1()
        .flex()
        .flex_col()
        .child(render_sub_nav(view, buckets_count, active_transfers, cx))
        .child(body)
        .into_any_element()
}

/// Horizontal tab strip (T038's `GitView` pattern — a documented deviation
/// from the mockup's left-column sidebar nav).
fn render_sub_nav(
    active: S3View,
    buckets_count: usize,
    active_transfers: usize,
    cx: &mut Context<S3Page>,
) -> impl IntoElement {
    let items = [
        (S3View::Explorer, 0),
        (S3View::Buckets, buckets_count),
        (S3View::Transfers, active_transfers),
        (S3View::Properties, 0),
    ];
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .p(px(4.))
        .m(px(12.))
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
                        .font_weight(FontWeight::BOLD)
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

/// Buckets view: `list_buckets()` result as its own list (design spec §4).
fn buckets_view(page: &S3Page, cx: &mut Context<S3Page>) -> AnyElement {
    if page.buckets_loading {
        return empty_state(cx, "Loading buckets…");
    }
    if page.buckets.is_empty() {
        return empty_state(cx, "No buckets.");
    }
    div()
        .flex_1()
        .flex()
        .flex_col()
        .children(page.buckets.iter().map(|b| {
            let name = b.name.clone();
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(16.))
                .py(px(10.))
                .cursor_pointer()
                .border_b_1()
                .border_color(theme::border(cx))
                .hover(|this| this.bg(theme::bg_hover(cx)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _ev, window, cx| {
                        this.open_bucket(&name, window, cx);
                    }),
                )
                .child(div().text_color(theme::fg(cx)).child(b.name.clone()))
                .into_any_element()
        }))
        .into_any_element()
}

/// Transfers view: Upload/Download actions (wired to `dialogs::pick_file`/
/// `pick_directory`, T039 gate 1) + the chunked job queue with per-row
/// progress and Cancel (design spec §1/§4).
fn transfers_view(page: &S3Page, cx: &mut Context<S3Page>) -> AnyElement {
    let can_upload = page.current_bucket_prefix(cx).is_some();
    let can_download = page.current_selection(cx).is_some();
    let has_finished = page.transfers.iter().any(|r| r.job.state.is_terminal());

    let toolbar = div()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(16.))
        .py(px(8.))
        .child(
            Button::new("s3-upload")
                .label("Upload")
                .when(!can_upload, |b| b.disabled(true))
                .on_click({
                    let this = cx.weak_entity();
                    move |_event, _window, cx| {
                        this.update(cx, |this, cx| this.start_upload(cx)).ok();
                    }
                }),
        )
        .child(
            Button::new("s3-download")
                .label("Download selected")
                .when(!can_download, |b| b.disabled(true))
                .on_click({
                    let this = cx.weak_entity();
                    move |_event, _window, cx| {
                        this.update(cx, |this, cx| this.start_download(cx)).ok();
                    }
                }),
        )
        .when(has_finished, |el| {
            el.child(
                Button::new("s3-clear-finished")
                    .label("Clear finished")
                    .on_click({
                        let this = cx.weak_entity();
                        move |_event, _window, cx| {
                            this.update(cx, |this, cx| this.clear_finished_transfers(cx))
                                .ok();
                        }
                    }),
            )
        });

    let list: AnyElement = if page.transfers.is_empty() {
        empty_state(cx, "No transfers yet.")
    } else {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .children(
                page.transfers
                    .iter()
                    .map(|row| render_transfer_row(row, cx)),
            )
            .into_any_element()
    };

    div()
        .flex_1()
        .flex()
        .flex_col()
        .child(toolbar)
        .when_some(page.view_error.clone(), |el, msg| {
            el.child(
                div()
                    .px(px(16.))
                    .pb(px(8.))
                    .text_sm()
                    .text_color(theme::danger(cx))
                    .child(msg),
            )
        })
        .child(list)
        .into_any_element()
}

fn render_transfer_row(row: &TransferRow, cx: &mut Context<S3Page>) -> AnyElement {
    let job = &row.job;
    let pct = if job.bytes_total > 0 {
        (job.bytes_done as f32 / job.bytes_total as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let status_label = match &job.state {
        TransferState::Queued => "queued".to_string(),
        TransferState::InProgress => format!("{:.0}%", pct * 100.0),
        TransferState::Completed => "done".to_string(),
        TransferState::Failed { reason } => format!("failed: {reason}"),
        TransferState::Cancelled => "cancelled".to_string(),
    };
    let job_id = job.id;
    let cancellable = matches!(job.state, TransferState::Queued | TransferState::InProgress);
    let arrow = if job.direction == TransferDirection::Upload {
        "↑"
    } else {
        "↓"
    };

    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .px(px(16.))
        .py(px(10.))
        .border_b_1()
        .border_color(theme::border(cx))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme::fg(cx))
                        .child(format!("{arrow} {}/{}", job.bucket, job.key)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::fg_secondary(cx))
                        .child(status_label),
                ),
        )
        .child(
            div()
                .h(px(5.))
                .rounded(px(3.))
                .bg(theme::bg_secondary(cx))
                .border_1()
                .border_color(theme::border(cx))
                .overflow_hidden()
                .child(
                    div()
                        .h_full()
                        .w(relative(pct))
                        .bg(theme::accent(cx)),
                ),
        )
        .when(cancellable, |el| {
            el.child(
                div().mt(px(2.)).child(
                    Button::new(SharedString::from(format!("s3-transfer-cancel-{job_id}")))
                        .label("Cancel")
                        .on_click({
                            let this = cx.weak_entity();
                            move |_event, _window, cx| {
                                this.update(cx, |this, cx| this.cancel_transfer(job_id, cx))
                                    .ok();
                            }
                        }),
                ),
            )
        })
        .into_any_element()
}

/// Properties view: selected-object metadata; honest empty state (T023
/// pattern) when nothing is selected.
fn properties_view(page: &S3Page, cx: &mut Context<S3Page>) -> AnyElement {
    let Some(entry) = page.current_selection(cx) else {
        return empty_state(cx, "Select an object in Explorer to see its properties.");
    };
    elevated_card(cx)
        .m(px(16.))
        .p(px(16.))
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(theme::fg(cx))
                .child(entry.name.clone()),
        )
        .child(property_row("Key", entry.path.clone(), cx))
        .child(property_row("Size", format_bytes(entry.size), cx))
        .child(property_row("Modified (unix)", entry.modified.to_string(), cx))
        .into_any_element()
}

fn property_row(key: &str, value: String, cx: &Context<S3Page>) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .gap(px(12.))
        .py(px(6.))
        .border_b_1()
        .border_color(theme::border(cx))
        .child(
            div()
                .text_sm()
                .text_color(theme::fg_secondary(cx))
                .child(key.to_string()),
        )
        .child(div().text_sm().text_color(theme::fg(cx)).child(value))
}

fn empty_state(cx: &Context<S3Page>, message: &str) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_color(theme::fg_secondary(cx))
                .text_sm()
                .child(message.to_string()),
        )
        .into_any_element()
}

/// Human-readable byte size (design spec's Transfers/Properties display).
/// Pure so it's unit-testable without a GPUI context.
fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

fn no_profiles_card(cx: &mut Context<S3Page>) -> AnyElement {
    elevated_card(cx)
        .p(px(32.0))
        .child("No S3 profiles configured")
        .child(
            div()
                .mt(px(12.0))
                .text_color(theme::fg_secondary(cx))
                .child("Add a profile in Settings to connect to your S3-compatible storage."),
        )
        .child(
            div().mt(px(16.0)).child(
                Button::new("open-settings")
                    .label("Open Settings")
                    .on_click(move |_event, window, cx| {
                        window.dispatch_action(NavigateToSettings.boxed_clone(), cx);
                    }),
            ),
        )
        .into_any_element()
}

fn credentials_form(page: &mut S3Page, cx: &mut Context<S3Page>) -> AnyElement {
    let default_profile = page.config.s3.default_profile.clone();
    let endpoint = page
        .config
        .s3
        .profiles
        .get(&default_profile)
        .map(|p| p.endpoint.as_str())
        .unwrap_or("unknown");

    elevated_card(cx)
        .p(px(24.0))
        .w(px(400.0))
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(theme::fg(cx))
                .child(format!("Connect to {default_profile}")),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_color(theme::fg_secondary(cx))
                .text_sm()
                .child(format!("Endpoint: {endpoint}")),
        )
        .child(
            div()
                .mt(px(16.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(
                    div().flex().flex_col().gap(px(4.0)).child(
                        div()
                            .text_color(theme::fg(cx))
                            .text_sm()
                            .child("Access Key ID"),
                    )
                    .child(Input::new(&page.access_key_input)),
                )
                .child(
                    div().flex().flex_col().gap(px(4.0)).child(
                        div()
                            .text_color(theme::fg(cx))
                            .text_sm()
                            .child("Secret Access Key"),
                    )
                    .child(Input::new(&page.secret_key_input).mask_toggle()),
                ),
        )
        .child(
            div().mt(px(16.0)).flex().gap(px(8.0)).child(
                Button::new("connect-btn")
                    .label("Connect")
                    .on_click({
                        let this = cx.weak_entity();
                        move |_event, window, cx: &mut App| {
                            this.update(cx, |this, cx| {
                                this.start_connect(window, cx);
                            })
                            .ok();
                        }
                    }),
            ),
        )
        .into_any_element()
}

fn connecting_card(cx: &App) -> AnyElement {
    elevated_card(cx)
        .p(px(32.0))
        .child("Connecting…")
        .child(
            div()
                .mt(px(12.0))
                .text_color(theme::fg_secondary(cx))
                .child("Storing credentials and connecting to S3…"),
        )
        .into_any_element()
}

fn error_card(message: String, cx: &App) -> AnyElement {
    elevated_card(cx)
        .p(px(32.0))
        .child(
            div()
                .text_color(theme::danger(cx))
                .child("Connection failed"),
        )
        .child(
            div()
                .mt(px(12.0))
                .text_color(theme::fg_secondary(cx))
                .text_sm()
                .child(message),
        )
        .into_any_element()
}

impl crate::Page for S3Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        <Self as Render>::render(self, window, cx).into_any_element()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]
mod tests {
    use super::S3Page;
    use super::S3State;
    use super::format_bytes;
    use crate::explorer::ExplorerPane;
    use crate::explorer::tests::CountingProvider;
    use chronos_fm_core::config::{Config, S3Config, S3Profile};
    use chronos_fm_services::fs::listing::FileEntryDto;
    use chronos_fm_services::s3;
    use gpui::{AppContext, TestAppContext};
    use std::sync::Arc;
    use std::time::Duration;

    /// A `Config` with one S3 profile (`rustfs`), mirroring the live-run
    /// `config.toml` used to reproduce T021 against local RustFS.
    fn config_with_profile() -> Config {
        Config {
            s3: S3Config {
                default_profile: "rustfs".to_string(),
                profiles: std::collections::BTreeMap::from([(
                    "rustfs".to_string(),
                    S3Profile {
                        endpoint: "http://127.0.0.1:9000".to_string(),
                        region: "us-east-1".to_string(),
                        force_path_style: true,
                    },
                )]),
            },
            ..Default::default()
        }
    }

    #[gpui::test]
    async fn connect_callback_wires_pane_and_kicks_off_bucket_listing(
        cx: &mut TestAppContext,
    ) {
        // T021 regression: the connect callback must cross the S3Page → pane
        // boundary — wiring the provider and setting the profile root has to
        // poll the provider immediately. The original `loaded = false` handoff
        // no-oped (`reload()` returns early for provider-backed panes, B.1),
        // and the first fix (`pane.downgrade().update_in`, commit `32cb1ac`)
        // failed silently with "entity has no current window" — the pane is
        // created via `cx.new` and never registered in the window-by-entity
        // map. This test drives `wire_pane` — the exact callback the connect
        // flow runs on `Ok(client)` — through `S3Page`, not the pane directly.
        cx.update(gpui_component::init);
        // The pane's listing path reads the explorer clipboard global for
        // cut-row dimming; register it like the explorer test harness does.
        cx.update(crate::explorer::clipboard::init);
        let provider = Arc::new(CountingProvider::new(vec![
            bucket("bucket-a"),
            bucket("bucket-b"),
        ]));

        let window = build_page_with_pane(cx);
        wire_pane_for_test(&window, provider.clone(), cx);

        // Drive the async provider listing spawned by `reload_provider`.
        cx.background_executor
            .timer(Duration::from_millis(50))
            .await;
        cx.run_until_parked();

        assert_eq!(
            provider.call_count(),
            1,
            "the provider is polled exactly once right after connect"
        );
        assert_browsing_state(&window, cx);
    }

    /// A fake bucket-listing entry.
    fn bucket(name: &str) -> FileEntryDto {
        FileEntryDto {
            name: name.to_string(),
            path: format!("s3://rustfs@{name}/"),
            kind: "dir".to_string(),
            size: 0,
            modified: 0,
        }
    }

    /// The same wiring the connect callback runs on `Ok(client)` (T021).
    fn wire_pane_for_test(
        window: &gpui::WindowHandle<S3Page>,
        provider: Arc<CountingProvider>,
        cx: &mut TestAppContext,
    ) {
        window
            .update(cx, |page, window, cx| {
                let pane = page.s3_pane.as_ref().expect("pane created").clone();
                page.wire_pane(
                    window,
                    &pane,
                    provider,
                    s3::s3_profile_root("rustfs"),
                    cx,
                );
            })
            .expect("window update");
    }

    /// Assert the pane reached the profile root with the buckets applied.
    fn assert_browsing_state(window: &gpui::WindowHandle<S3Page>, cx: &mut TestAppContext) {
        window
            .read_with(cx, |page, cx| {
                assert!(matches!(page.state, S3State::Browsing));
                let pane = page.s3_pane.as_ref().expect("pane created");
                assert_eq!(pane.read(cx).cwd, "s3://rustfs@");
                assert_eq!(pane.read(cx).entries.len(), 2, "buckets listed");
                assert!(pane.read(cx).entries.iter().any(|e| e.name == "bucket-a"));
            })
            .expect("window read");
    }

    /// Build an `S3Page` with an embedded pane created exactly as
    /// `start_connect` creates it (via `cx.new`, with `loaded = true` so the
    /// local FS is never listed through the pane's render path).
    ///
    /// Kept in a plain helper like the explorer test harness (`new_explorer`)
    /// for readability. Note the module must NOT `use super::*`: that glob
    /// imports `gpui::test` (a proc macro re-exported under `test-support`)
    /// into this scope, and the builtin `#[test]` emitted inside the
    /// `#[gpui::test]` expansion then resolves to the proc macro itself —
    /// infinite self-expansion → recursion limit (T021 report §1.4). Use
    /// explicit imports instead.
    fn build_page_with_pane(cx: &mut TestAppContext) -> gpui::WindowHandle<S3Page> {
        cx.add_window(|window, cx| {
            let mut page = S3Page::new(config_with_profile(), window, cx);
            let pane = cx.new(|cx| {
                let mut p = ExplorerPane::build(None, window, cx);
                p.loaded = true;
                p
            });
            page.s3_pane = Some(pane.clone());
            page
        })
    }

    #[test]
    fn format_bytes_under_1024_is_plain_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn format_bytes_scales_through_units() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(1024u64.pow(4)), "1.0 TB");
    }

    #[test]
    fn format_bytes_caps_at_tb_for_huge_sizes() {
        // No PB unit in the table — the largest unit (TB) keeps growing
        // instead of indexing out of bounds.
        assert_eq!(format_bytes(1024u64.pow(5)), "1024.0 TB");
    }
}
