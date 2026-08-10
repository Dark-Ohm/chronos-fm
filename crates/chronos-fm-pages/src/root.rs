//! Root view of the Explorer window — one of the app's two top-level "pillars"
//! (the other being the launcher window, `chronos-fm-launcher`, added in P3).
//!
//! This lives in `chronos-fm-pages` rather than the binary because it is the
//! Explorer pillar's window root: it owns the page entities and routes between
//! them, depending only downward on `chronos-fm-ui` (chrome) and `chronos-fm-services`
//! (search). The binary just opens a window hosting this view; the future
//! launcher window will be a symmetric root in `chronos-fm-launcher`.

use crate::explorer::ExplorerPage;
use crate::s3::NavigateToSettings;
use crate::{
    PageKind, extensions::ExtensionsPage, git::GitPage, s3::S3Page, settings::SettingsPage,
};
use gpui::{
    AnyElement, App, AsyncWindowContext, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, Render, WeakEntity, Window, div, prelude::*, px,
};
use gpui_component::resizable::ResizableState;
use gpui_component::{Icon, Root, Sizable, Theme, ThemeMode as GpuiThemeMode};
use chronos_fm_core::config::{self, Config, ConfigOverride, ConfigWatcher};
use chronos_fm_core::telemetry::LogErr;
use chronos_fm_services::search::SearchService;
use chronos_fm_store::KvStore;
use chronos_fm_ui::components::layout::footer::{FooterProps, footer};
use chronos_fm_ui::components::layout::unified_toolbar::{
    AccountMenuAction, AccountMenuCommand, UnifiedToolbarProps, unified_toolbar,
};
use chronos_fm_ui::theme::theme;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;
use tracing::info;

/// The application root view that hosts the page sidebar, the active page, and
/// the shared configuration and search state.
pub struct RootView {
    current_page: PageKind,
    focus_handle: FocusHandle,
    // Page entities
    explorer: Entity<ExplorerPage>,
    git: Entity<GitPage>,
    s3: Entity<S3Page>,
    extensions: Entity<ExtensionsPage>,
    settings: Entity<SettingsPage>,
    search_service: Option<Arc<SearchService>>,
    indexing_progress: Option<f32>,
    // Currently-applied configuration and the inputs needed to recompute it on
    // hot reload: the file path and the env/CLI override layers that sit above
    // the file (config.md §3).
    config: Config,
    config_path: PathBuf,
    config_overrides: Vec<ConfigOverride>,
    // Latest config load error, surfaced in the footer. Held here (not in the
    // explorer's transient status) so it is not cleared by an explorer
    // directory reload and survives across pages.
    config_status: Option<String>,
    // False until `apply_config` has pushed a theme into gpui at least once.
    // The mode/accent branches below are diff-driven (hot reload only wants to
    // re-theme on an actual change), but a diff against `Config::default()` is
    // silent when the loaded config *equals* the default — which is exactly the
    // common case now that the product default is dark (T037). Without this
    // flag the registry's own default (light) would survive startup and no
    // dark-mode user would ever see dark on first paint.
    theme_applied: bool,
    // Kept alive for the window's lifetime so the OS watch is not dropped.
    _config_watcher: Option<ConfigWatcher>,
}

impl RootView {
    /// Build the Explorer window root: instantiate the page entities and start
    /// the indexing-progress poll. `resizable` is created at the application
    /// level and the search service is initialized by the binary (it owns the
    /// async runtime), keeping `chronos-fm-pages` free of runtime concerns.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resizable: Entity<ResizableState>,
        search_service: Option<Arc<SearchService>>,
        store: Option<Arc<dyn KvStore>>,
        config: Config,
        config_path: PathBuf,
        config_overrides: Vec<ConfigOverride>,
        config_error: Option<String>,
        // T046 residual: `--page=<name>` debug flag, so a vision/visual-proof
        // pass doesn't need working interactive input just to reach a
        // non-default page. `None` (the flag omitted) keeps the product
        // default (Explorer) — this never changes normal-user behavior.
        initial_page: Option<PageKind>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        let restore_tabs = config.explorer.restore_tabs;
        let explorer = cx.new(|cx| {
            ExplorerPage::new(
                resizable,
                search_service.clone(),
                store,
                restore_tabs,
                window,
                cx,
            )
        });
        let git = cx.new(|cx| GitPage::new(explorer.downgrade(), window, cx));
        let s3 = cx.new(|cx| S3Page::new(config.clone(), window, cx));
        let extensions = cx.new(|cx| ExtensionsPage::new(config.clone(), window, cx));
        let settings = cx.new(|cx| SettingsPage::new(config.clone(), window, cx));

        let mut view = RootView {
            current_page: initial_page.unwrap_or(PageKind::Explorer),
            focus_handle,
            explorer,
            git,
            s3,
            extensions,
            settings,
            search_service,
            indexing_progress: Some(1.0), // Start as hidden/done
            // Start from defaults so the initial `apply_config` below treats the
            // loaded config as a change and applies theme + ui uniformly.
            config: Config::default(),
            config_path,
            config_overrides,
            config_status: None,
            theme_applied: false,
            _config_watcher: None,
        };
        view.start_progress_loop(window, cx);
        view.apply_config(config, config_error, window, cx);
        view.start_config_watch(window, cx);
        view
    }

    /// Apply a freshly-merged configuration to the live UI: switch the theme
    /// mode, propagate `[ui]` settings to the explorer, and surface any load
    /// error in the status bar. Safe to call repeatedly (hot reload).
    pub fn apply_config(
        &mut self,
        config: Config,
        config_error: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.theme_applied || config.theme.mode != self.config.theme.mode {
            let mode = match config.theme.mode {
                config::ThemeMode::Light => GpuiThemeMode::Light,
                config::ThemeMode::Dark => GpuiThemeMode::Dark,
                // `system` follows the OS appearance reported by the window.
                config::ThemeMode::System => GpuiThemeMode::from(window.appearance()),
            };
            Theme::change(mode, Some(window), cx);
        }

        // Accent: re-seed the active theme configs' accent-derived colours so
        // the whole UI re-colors (buttons, selection, caret, links). Both modes
        // are seeded so a later mode switch keeps the chosen accent.
        if config.theme.accent != self.config.theme.accent && cx.has_global::<Theme>() {
            let hex = config::accent_hex(&config.theme.accent);
            // Translucent selection, matching the default `#007acc38`.
            let selection = format!("{hex}33");
            let mode = Theme::global(cx).mode;
            {
                let theme = Theme::global_mut(cx);
                let light = Rc::make_mut(&mut theme.light_theme);
                let dark = Rc::make_mut(&mut theme.dark_theme);
                for theme_config in [light, dark] {
                    let colors = &mut theme_config.colors;
                    // Solid accent keys.
                    colors.accent = Some(hex.clone().into());
                    colors.primary = Some(hex.clone().into());
                    colors.caret = Some(hex.clone().into());
                    colors.link = Some(hex.clone().into());
                    colors.ring = Some(hex.clone().into());
                    colors.progress_bar = Some(hex.clone().into());
                    colors.drag_border = Some(hex.clone().into());
                    colors.selection = Some(selection.clone().into());
                    // Clear hover/active shades so the theme's own fallbacks
                    // derive them from the new primary/link, instead of leaving
                    // the JSON's hardcoded blue variants clashing with a
                    // non-blue accent.
                    colors.primary_hover = None;
                    colors.primary_active = None;
                    colors.link_hover = None;
                    colors.link_active = None;
                    colors.drop_target = None;
                }
            }
            Theme::change(mode, Some(window), cx);
        }

        self.theme_applied = true;

        // Condense the (possibly multi-line) diagnostic to a single line plus the
        // file path for the one-line status bar; full detail is in the logs.
        self.config_status = config_error.as_ref().map(|error| {
            let summary = error.lines().next().unwrap_or(error.as_str());
            format!("config: {summary} ({})", self.config_path.display())
        });

        let ui = config.ui.clone();
        let explorer_cfg = config.explorer.clone();
        self.explorer.update(cx, |page, cx| {
            page.apply_config_ui(&ui, cx);
            page.apply_config_explorer(&explorer_cfg, cx);
        });
        // Keep the Settings page's controls in sync with the on-disk config:
        // without this its active mode/sort/switch states stay frozen at the
        // construction snapshot after the first hot reload.
        self.settings.update(cx, |page, _cx| {
            page.set_config(config.clone());
        });
        self.s3.update(cx, |page, _cx| {
            page.set_config(config.clone());
        });

        self.extensions.update(cx, |page, _cx| {
            page.set_config(config.clone());
        });

        self.config = config;
        cx.notify();
    }

    /// Watch `config.toml` and re-apply on change. The `notify` callback runs on
    /// a background thread and only pings a channel; a foreground poll loop does
    /// the reload so all entity updates happen on the GPUI thread.
    fn start_config_watch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (sender, receiver) = mpsc::channel::<()>();
        match ConfigWatcher::new(&self.config_path, move || {
            // A closed channel just means the app is shutting down.
            sender.send(()).log_err();
        }) {
            Ok(watcher) => self._config_watcher = Some(watcher),
            Err(error) => {
                tracing::warn!("config hot reload disabled: {error}");
                return;
            }
        }

        cx.spawn_in(
            window,
            move |this: WeakEntity<RootView>, cx: &mut AsyncWindowContext| {
                let mut cx = cx.clone();
                async move {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(400))
                            .await;
                        let mut changed = false;
                        let mut disconnected = false;
                        loop {
                            match receiver.try_recv() {
                                Ok(()) => changed = true,
                                Err(mpsc::TryRecvError::Empty) => break,
                                // The watcher was dropped; stop polling rather
                                // than spinning every 400ms forever.
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
                        let update = this.update_in(&mut cx, |this, window, cx| {
                            let (mut config, diagnostics) =
                                config::load_from_path(&this.config_path);
                            for over in &this.config_overrides {
                                config.apply_override(over);
                            }
                            let config_error = config::report_diagnostics(&diagnostics);
                            this.apply_config(config, config_error, window, cx);
                        });
                        if update.is_err() {
                            break; // The view (and window) is gone.
                        }
                    }
                }
            },
        )
        .detach();
    }

    /// Switches the active page, notifying for a redraw only if it changed.
    pub fn set_page(&mut self, page: PageKind, cx: &mut Context<Self>) {
        if self.current_page != page {
            self.current_page = page;
            cx.notify();
        }
    }

    fn start_progress_loop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Without a search service there is no indexing progress to poll, so avoid
        // rescheduling a per-frame no-op forever.
        if self.search_service.is_none() {
            return;
        }

        if let Some(progress) = self.check_progress_update() {
            self.indexing_progress = Some(progress);
            cx.notify();
        }

        // Poll every frame (simple and effective for this case)
        cx.on_next_frame(window, |view: &mut RootView, window, cx| {
            view.start_progress_loop(window, cx);
        });
    }

    fn check_progress_update(&self) -> Option<f32> {
        let service = self.search_service.as_ref()?;
        let rx = service.progress_subscription();
        // Just read current value
        let val = *rx.borrow();
        Some(val)
    }
}

impl Focusable for RootView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let toolbar = unified_toolbar(
            UnifiedToolbarProps {
                account_name: "syuya2036".to_string(),
                account_plan: "Free".to_string(),
            },
            cx,
        );

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::bg(cx))
            .relative()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_account_action))
            .on_action(cx.listener(Self::handle_navigate_to_settings))
            .child(toolbar)
            .child(
                // Main content: toolbar + page
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .min_h(px(0.0))
                    .relative()
                    .child(
                        // Main content area - render active page
                        // (painted first = behind the nav rail)
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .pl(px(64.0))
                            .child(self.render_active_page(window, cx)),
                    )
                    .child(
                        // Left navigation toolbar — ABSOLUTE, painted ON TOP
                        self.render_navigation(cx),
                    ),
            )
            .child(
                // Footer status bar
                {
                    // A config load error takes precedence over the explorer's
                    // transient status and is always shown as an error.
                    let (status_message, status_is_error) = match &self.config_status {
                        Some(message) => (Some(message.clone()), true),
                        None => match self.explorer.read(cx).status_for_footer(cx) {
                            Some((text, is_error)) => (Some(text), is_error),
                            None => (None, false),
                        },
                    };
                    let (selected_count, total_count) = self.explorer.read(cx).selection_counts(cx);
                    let current_path = self.explorer.read(cx).current_path(cx);
                    let props = FooterProps {
                        selected_count,
                        total_count,
                        current_path,
                        indexing_progress: self.indexing_progress,
                        status_message,
                        status_is_error,
                        ..Default::default()
                    };
                    footer(props, cx)
                },
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

impl RootView {
    fn handle_navigate_to_settings(
        &mut self,
        _action: &NavigateToSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_page(PageKind::Settings, cx);
    }

    fn handle_account_action(
        &mut self,
        action: &AccountMenuAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action.command {
            AccountMenuCommand::ProfileSummary => {
                window.prevent_default();
            }
            AccountMenuCommand::Settings => self.set_page(PageKind::Settings, cx),
            AccountMenuCommand::Extensions => self.set_page(PageKind::Extensions, cx),
            AccountMenuCommand::Keymap
            | AccountMenuCommand::Themes
            | AccountMenuCommand::IconThemes => {
                info!(?action.command, "Account menu item not yet implemented");
                window.prevent_default();
            }
            AccountMenuCommand::SignOut => {
                info!("Sign out requested");
            }
        }
    }

    fn render_navigation(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let active_page = self.current_page;

        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .w(px(64.0))
            .flex()
            .flex_col()
            .items_center()
            .bg(theme::toolbar_bg(cx))
            .border_r_1()
            .border_color(theme::toolbar_border(cx))
            .py(px(16.0))
            .child(
                // Page navigation buttons
                div().flex().flex_col().items_center().gap_2().children(
                    PageKind::all().into_iter().map(|page| {
                        let is_active = active_page == page;
                        self.navigation_button(page, is_active, cx)
                    }),
                ),
            )
    }

    fn navigation_button(
        &self,
        page: PageKind,
        active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        div()
            .id(("nav-btn", page as usize))
            .w(px(48.0))
            .h(px(48.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(8.0))
            .cursor_pointer()
            .when(active, |this| {
                this.bg(theme::toolbar_active_bg(cx)).shadow_sm()
            })
            .when(!active, |this| {
                this.hover(|style| style.bg(theme::toolbar_hover(cx)))
            })
            .on_click(cx.listener(move |view, _event, _window, cx| {
                view.set_page(page, cx);
            }))
            .child(
                // Mockup §2.3: nav glyphs are 19px in a 48×48 chip.
                Icon::new(Icon::empty())
                    .path(page.icon_path())
                    .with_size(px(19.0))
                    .text_color(if active {
                        theme::toolbar_active_text(cx)
                    } else {
                        theme::toolbar_text(cx)
                    }),
            )
    }

    fn render_active_page(&self, _window: &mut Window, _cx: &mut Context<Self>) -> AnyElement {
        match self.current_page {
            PageKind::Explorer => self.explorer.clone().into_any_element(),

            PageKind::Git => self.git.clone().into_any_element(),
            PageKind::S3 => self.s3.clone().into_any_element(),
            PageKind::Extensions => self.extensions.clone().into_any_element(),
            PageKind::Settings => self.settings.clone().into_any_element(),
        }
    }
}
