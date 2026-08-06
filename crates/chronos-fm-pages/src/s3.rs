//! S3 page — object-storage browser with interactive credentials flow
//! and embedded ExplorerPane for bucket/prefix navigation (T011).

use std::sync::Arc;

use chronos_fm_core::config::Config;
use chronos_fm_core::config::s3_credentials::S3CredentialsManager;
use chronos_fm_services::fs::provider::FileSystemProvider;
use chronos_fm_services::s3::{self, S3Client};
use chronos_fm_ui::patterns::elevated_card;
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};

use crate::explorer::ExplorerPane;

/// Which phase the S3 page is in.
enum S3State {
    NoProfiles,
    NeedCredentials,
    Connecting,
    Browsing,
    Error { message: String },
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
}

impl Focusable for S3Page {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl S3Page {
    pub fn new(config: Config, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let access_key_input = cx.new(|cx| InputState::new(window, cx));
        let secret_key_input = cx.new(|cx| InputState::new(window, cx));
        let state = Self::derive_state(&config);
        Self {
            config,
            focus_handle: cx.focus_handle(),
            access_key_input,
            secret_key_input,
            state,
            s3_pane: None,
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
        S3State::Browsing | S3State::Connecting => {
            if let Some(pane) = &page.s3_pane {
                return div()
                    .flex_1()
                    .relative()
                    .child(pane.clone())
                    .when(matches!(page.state, S3State::Connecting), |d| {
                        d.child(
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
                    })
                    .into_any_element();
            }
            // Fallthrough: show connecting card if pane not created yet.
            connecting_card(cx)
        }
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

fn no_profiles_card(cx: &App) -> AnyElement {
    elevated_card(cx)
        .p(px(32.0))
        .child("No S3 profiles configured")
        .child(
            div()
                .mt(px(12.0))
                .text_color(theme::fg_secondary(cx))
                .child("Add a profile in Settings to connect to your S3-compatible storage."),
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
                    .child(Input::new(&page.secret_key_input)),
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
}
