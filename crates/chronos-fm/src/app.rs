//! Application entry / startup sequence.
//!
//! The binary is the only layer allowed to depend on every crate, so it wires
//! the pillars together: it builds the shared window chrome from `chronos-fm-ui`,
//! initializes services that need the async runtime, and opens the Explorer
//! window hosting `chronos_fm_pages::RootView`. The launcher window (`chronos-fm-launcher`,
//! P3) will be opened from here too, as a symmetric second pillar.

use crate::cli::Cli;
use futures::StreamExt;
use gpui::{App, AppContext, AsyncApp, Bounds, BorrowAppContext, px, size};
use gpui_component::resizable::ResizableState;
use gpui_component::{Root, Theme, ThemeRegistry};
use chronos_fm_core::config::{self, ConfigOverride};
use chronos_fm_core::telemetry::logging::init_logging;
use chronos_fm_pages::RootView;
use chronos_fm_services::devices::UDisks2Backend;
use chronos_fm_services::search::SearchService;
use chronos_fm_store::{KvStore, RedbKvStore, StoreLogConfig};
use chronos_fm_ui::assets::Assets;
use chronos_fm_ui::components::layout::unified_toolbar::UNIFIED_TOOLBAR_HEIGHT;
use chronos_fm_ui::window::{self, traffic_lights::TrafficLightsHook};
use std::sync::Arc;

pub struct ChronosFmApp;

impl ChronosFmApp {
    pub fn run(cli: &Cli) {
        init_logging();

        // Load configuration before opening the window: defaults < file < env <
        // CLI (config.md §3). A missing file is created with defaults so users
        // have something to edit and the watcher has a target. Parse failures are
        // non-fatal — we fall back to defaults and surface the error in the UI.
        let config_path = config::paths::config_file();
        if let Err(error) = config::ensure_exists(&config_path) {
            tracing::warn!("could not create {}: {error}", config_path.display());
        }
        let (mut config, diagnostics) = config::load_from_path(&config_path);
        let config_overrides = vec![ConfigOverride::from_env(), cli.overrides()];
        for over in &config_overrides {
            config.apply_override(over);
        }
        let config_error = config::report_diagnostics(&diagnostics);

        // GPUI drives the app (replacing `#[tokio::main]`; ADR 0004, async-runtime.md
        // §7). The search service is now tokio-free — its file watcher and progress
        // channels run on std threads and runtime-agnostic channels — so no async
        // runtime needs to be entered here.
        // gpui-ce fork: Application::new() is not public; platform selection
        // (wayland/x11, fonts) lives in gpui_platform — same bootstrap as ChronOS.
        gpui_platform::application()
            .with_assets(Assets)
            .run(move |app: &mut App| {
            gpui_component::init(app);
            activate_chronos_theme(app);

            // Initialize the removable-media device store global and start the
            // background hotplug watcher (non-blocking — if udisks2 is unreachable,
            // the Devices section just renders nothing). The watcher also
            // registers the live backend as a global (T008) so sidebar clicks
            // can mount/unmount without a new D-Bus connection.
            chronos_fm_ui::devices_store::init(app);
            spawn_device_hotplug_watcher(app);

            let resizable = app.new(|_| ResizableState::default());
            let bounds = Bounds::centered(
                None,
                size(px(config::WINDOW_WIDTH), px(config::WINDOW_HEIGHT)),
                app,
            );
            let traffic_lights = TrafficLightsHook::new().center_vertically(UNIFIED_TOOLBAR_HEIGHT);
            let window_options = window::unified_window_options(bounds, &traffic_lights);

            // Open the host KV store (`state.redb`) for tab/session restore
            // (docs/persistence.md §3). Failure is non-fatal: the app starts
            // without session persistence rather than crashing.
            let store: Option<Arc<dyn KvStore>> = open_host_store();

            let opened = app.open_window(window_options, {
                let config = config.clone();
                let config_path = config_path.clone();
                let config_overrides = config_overrides.clone();
                let config_error = config_error.clone();
                let store = store.clone();
                move |window, cx| {
                    // Initialize SearchService. Failure is non-fatal: the app starts
                    // with full-text search disabled rather than crashing.
                    let search_service: Option<Arc<SearchService>> = match SearchService::new() {
                        Ok(service) => Some(Arc::new(service)),
                        Err(e) => {
                            tracing::error!(
                                "Failed to initialize search service; starting with search disabled: {}",
                                e
                            );
                            None
                        }
                    };

                    // Kick off initial indexing on GPUI's background executor, which
                    // is a thread pool (replacing tokio::task::spawn_blocking;
                    // async-runtime.md §2).
                    if let Some(service) = &search_service {
                        if let Some(job) = service.take_initial_indexing_job() {
                            cx.background_spawn(async move { job.run() }).detach();
                        }
                    }

                    let view = cx.new(|cx| {
                        RootView::new(
                            resizable.clone(),
                            search_service,
                            store,
                            config,
                            config_path,
                            config_overrides,
                            config_error,
                            window,
                            cx,
                        )
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                }
            });
            if let Err(error) = opened {
                tracing::error!("failed to open main window: {error}");
            }
        });
    }
}

/// Connects to `udisks2` and keeps `DeviceStore` live: an initial listing
/// on success, then a subscription to `InterfacesAdded`/`InterfacesRemoved`
/// that re-lists on every signal (simpler and more robust than diffing
/// individual signal payloads, and the list is cheap — a handful of
/// devices). If `udisks2` is unreachable (no system bus, container,
/// headless CI), logs a warning and leaves `DeviceStore` empty — the
/// Devices sidebar section (Task 6) just renders nothing, not an error.
fn spawn_device_hotplug_watcher(cx: &mut App) {
    cx.spawn(async move |cx: &mut AsyncApp| {
        let backend = match cx
            .background_spawn(async { UDisks2Backend::connect().await })
            .await
        {
            Ok(backend) => Arc::new(backend),
            Err(error) => {
                tracing::warn!("Devices panel unavailable: {error}");
                return;
            }
        };

        let backend_for_list: Arc<dyn chronos_fm_services::devices::DeviceBackend> =
            backend.clone();

        // Expose the backend to the UI layer (T008): the sidebar's click
        // handlers call `DeviceStore::mount_and_navigate`/`unmount` with this
        // handle instead of creating a fresh D-Bus connection per click.
        cx.update(|cx| {
            cx.update_global::<chronos_fm_ui::devices_store::DeviceStore, _>(|store, _cx| {
                store.backend = Some(backend_for_list.clone());
            });
        });

        // Initial snapshot.
        refresh_devices(cx, backend_for_list.clone()).await;

        // Live hotplug: udisks2's ObjectManager emits InterfacesAdded/
        // InterfacesRemoved on the same object (/org/freedesktop/UDisks2)
        // this module already talks to; re-list on every signal rather
        // than parsing the signal payload incrementally.
        let connection = backend.connection().clone();
        let proxy = match zbus::fdo::ObjectManagerProxy::new(
            &connection,
            "org.freedesktop.UDisks2",
            "/org/freedesktop/UDisks2",
        )
        .await
        {
            Ok(proxy) => proxy,
            Err(error) => {
                tracing::warn!("Devices hotplug subscription unavailable: {error}");
                return;
            }
        };

        let Ok(mut added) = proxy.receive_interfaces_added().await else {
            return;
        };
        let Ok(mut removed) = proxy.receive_interfaces_removed().await else {
            return;
        };

        loop {
            futures::select_biased! {
                _ = added.next() => refresh_devices(cx, backend_for_list.clone()).await,
                _ = removed.next() => refresh_devices(cx, backend_for_list.clone()).await,
                complete => break,
            }
        }
    })
    .detach();
}

async fn refresh_devices(
    cx: &mut AsyncApp,
    backend: Arc<dyn chronos_fm_services::devices::DeviceBackend>,
) {
    let result = cx.background_spawn(async move { backend.list().await }).await;
    cx.update(|cx| {
        cx.update_global::<chronos_fm_ui::devices_store::DeviceStore, _>(|store, _cx| {
            if let Ok(devices) = result {
                store.devices = devices;
            }
        });
    });
}

/// Load the Chronos theme set and make it the active light/dark palette.
///
/// `gpui_component::init` already registered its own default themes; this swaps
/// the active `light_theme` / `dark_theme` configs for the Chronos ones so every
/// surface (and native gpui-component widgets) follows the Chronos palette and
/// the runtime `config.theme.mode` toggle wired in `RootView::apply_config`.
fn activate_chronos_theme(app: &mut App) {
    {
        let registry = ThemeRegistry::global_mut(app);
        if let Err(err) = registry.load_themes_from_str(include_str!(
            "../assets/themes/chronos.theme.json"
        )) {
            tracing::error!("failed to load Chronos theme: {err}");
        }
    }

    let (light, dark) = {
        let registry = ThemeRegistry::global(app);
        (
            registry.themes().get("Chronos Light").cloned(),
            registry.themes().get("Chronos Dark").cloned(),
        )
    };

    if let (Some(light), Some(dark)) = (light, dark) {
        let mode = Theme::global(app).mode;
        {
            let theme = Theme::global_mut(app);
            theme.light_theme = light;
            theme.dark_theme = dark;
        }
        // Re-apply the current mode against the newly-activated palette.
        Theme::change(mode, None, app);
    } else {
        tracing::warn!("Chronos Light/Dark themes not found in registry");
    }
}

/// Opens the host KV store at `<data_dir>/state.redb`, creating the data
/// directory if needed. Returns `None` (and logs) on any failure so a broken or
/// unwritable store degrades to "no session persistence" rather than a crash.
fn open_host_store() -> Option<Arc<dyn KvStore>> {
    let data_dir = config::paths::data_dir();
    if let Err(error) = std::fs::create_dir_all(&data_dir) {
        tracing::error!("could not create data dir {}: {error}", data_dir.display());
        return None;
    }
    let path = data_dir.join("state.redb");
    match RedbKvStore::open(&path, &StoreLogConfig::default()) {
        Ok(store) => Some(Arc::new(store)),
        Err(error) => {
            tracing::error!("could not open KV store {}: {error}", path.display());
            None
        }
    }
}
