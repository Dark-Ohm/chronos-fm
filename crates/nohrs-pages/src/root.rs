//! Root view of the Explorer window — one of the app's two top-level "pillars"
//! (the other being the launcher window, `nohrs-launcher`, added in P3).
//!
//! This lives in `nohrs-pages` rather than the binary because it is the
//! Explorer pillar's window root: it owns the page entities and routes between
//! them, depending only downward on `nohrs-ui` (chrome) and `nohrs-services`
//! (search). The binary just opens a window hosting this view; the future
//! launcher window will be a symmetric root in `nohrs-launcher`.

use crate::explorer::ExplorerPage;
use crate::{
    extensions::ExtensionsPage, git::GitPage, s3::S3Page, settings::SettingsPage, PageKind,
};
use gpui::{
    div, prelude::*, px, rgb, AnyElement, App, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, Render, Window,
};
use gpui_component::input::InputState;
use gpui_component::resizable::ResizableState;
use gpui_component::{Icon, Root};
use nohrs_services::search::SearchService;
use nohrs_ui::components::layout::footer::{footer, FooterProps};
use nohrs_ui::components::layout::unified_toolbar::{
    unified_toolbar, AccountMenuAction, AccountMenuCommand, UnifiedToolbarProps,
};
use nohrs_ui::theme::theme;
use std::sync::Arc;
use tracing::info;

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
}

impl RootView {
    /// Build the Explorer window root: instantiate the page entities and start
    /// the indexing-progress poll. `resizable` is created at the application
    /// level and the search service is initialized by the binary (it owns the
    /// async runtime), keeping `nohrs-pages` free of runtime concerns.
    pub fn new(
        resizable: Entity<ResizableState>,
        search_service: Option<Arc<SearchService>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx));
        let focus_handle = cx.focus_handle();

        let explorer = cx.new(|cx| {
            ExplorerPage::new(
                resizable,
                search_input.clone(),
                search_service.clone(),
                cx.focus_handle(),
            )
        });
        let git = cx.new(|_cx| GitPage::new());
        let s3 = cx.new(|_cx| S3Page::new());
        let extensions = cx.new(|_cx| ExtensionsPage::new());
        let settings = cx.new(|_cx| SettingsPage::new());

        let mut view = RootView {
            current_page: PageKind::Explorer,
            focus_handle,
            explorer,
            git,
            s3,
            extensions,
            settings,
            search_service,
            indexing_progress: Some(1.0), // Start as hidden/done
        };
        view.start_progress_loop(window, cx);
        view
    }

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
            .bg(rgb(theme::BG))
            .relative()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_account_action))
            .child(toolbar)
            .child(
                // Main content: toolbar + page
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .min_h(px(0.0))
                    .child(
                        // Left navigation toolbar
                        self.render_navigation(cx),
                    )
                    .child(
                        // Main content area - render active page
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .child(self.render_active_page(window, cx)),
                    ),
            )
            .child(
                // Footer status bar
                {
                    let (status_message, status_is_error) =
                        match self.explorer.read(cx).status_for_footer() {
                            Some((text, is_error)) => (Some(text), is_error),
                            None => (None, false),
                        };
                    let props = FooterProps {
                        indexing_progress: self.indexing_progress,
                        status_message,
                        status_is_error,
                        ..Default::default()
                    };
                    footer(props, cx)
                },
            )
            .children(Root::render_modal_layer(window, cx))
            .children(Root::render_drawer_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

impl RootView {
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

    fn render_navigation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active_page = self.current_page;

        div()
            .w(px(64.0))
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .bg(rgb(theme::TOOLBAR_BG))
            .border_r_1()
            .border_color(rgb(theme::TOOLBAR_BORDER))
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
    ) -> impl IntoElement {
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
                this.bg(rgb(theme::TOOLBAR_ACTIVE_BG)).shadow_sm()
            })
            .when(!active, |this| {
                this.hover(|style| style.bg(rgb(theme::TOOLBAR_HOVER)))
            })
            .on_click(cx.listener(move |view, _event, _window, cx| {
                view.set_page(page, cx);
            }))
            .child(
                Icon::new(Icon::empty())
                    .path(page.icon_path())
                    .size_5()
                    .text_color(rgb(if active {
                        theme::TOOLBAR_ACTIVE_TEXT
                    } else {
                        theme::TOOLBAR_TEXT
                    })),
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
