//! S3 page — object-storage browser with interactive credentials flow
//! and embedded ExplorerPane for bucket/prefix navigation (T011).

use std::sync::Arc;

use chronos_fm_core::config::Config;
use chronos_fm_core::config::s3_credentials::S3CredentialsManager;
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
                        let _ = this.update_in(&mut cx, |this, _window, cx| {
                            // Wire the S3 provider into the pane and point it
                            // at the profile root (bucket listing).
                            pane.update(cx, |pane, cx| {
                                pane.set_provider(client.clone());
                                pane.cwd = s3_root.clone();
                                pane.loaded = false;
                                cx.notify();
                            });
                            this.state = S3State::Browsing;
                            cx.notify();
                        });
                    }
                    Err(error) => {
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
