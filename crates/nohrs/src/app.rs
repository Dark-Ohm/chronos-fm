//! Application entry / startup sequence.
//!
//! The binary is the only layer allowed to depend on every crate, so it wires
//! the pillars together: it builds the shared window chrome from `nohrs-ui`,
//! initializes services that need the async runtime, and opens the Explorer
//! window hosting `nohrs_pages::RootView`. The launcher window (`nohrs-launcher`,
//! P3) will be opened from here too, as a symmetric second pillar.

use gpui::{px, size, App, AppContext, Application, Bounds};
use gpui_component::resizable::ResizableState;
use gpui_component::Root;
use nohrs_core::config;
use nohrs_core::telemetry::logging::init_logging;
use nohrs_pages::RootView;
use nohrs_services::search::SearchService;
use nohrs_ui::assets::Assets;
use nohrs_ui::components::layout::unified_toolbar::UNIFIED_TOOLBAR_HEIGHT;
use nohrs_ui::window::{self, traffic_lights::TrafficLightsHook};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::task;

pub struct NohrsApp;

impl NohrsApp {
    pub fn run() {
        init_logging();

        Application::new().with_assets(Assets).run(|app: &mut App| {
            gpui_component::init(app);
            let resizable = ResizableState::new(app);
            let bounds = Bounds::centered(
                None,
                size(px(config::WINDOW_WIDTH), px(config::WINDOW_HEIGHT)),
                app,
            );
            let traffic_lights = TrafficLightsHook::new().center_vertically(UNIFIED_TOOLBAR_HEIGHT);
            let window_options = window::unified_window_options(bounds, &traffic_lights);

            let opened = app.open_window(window_options, |window, cx| {
                // Initialize SearchService. Failure is non-fatal: the app starts
                // with full-text search disabled rather than crashing.
                let handle = Handle::current();
                let search_service: Option<Arc<SearchService>> =
                    task::block_in_place(move || match handle.block_on(SearchService::new()) {
                        Ok(service) => Some(Arc::new(service)),
                        Err(e) => {
                            tracing::error!(
                                "Failed to initialize search service; starting with search disabled: {}",
                                e
                            );
                            None
                        }
                    });

                let view =
                    cx.new(|cx| RootView::new(resizable.clone(), search_service, window, cx));
                cx.new(|cx| Root::new(view.into(), window, cx))
            });
            if let Err(error) = opened {
                tracing::error!("failed to open main window: {error}");
            }
        });
    }
}
