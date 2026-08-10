/// Platform traffic-light (window control button) configuration and hooks.
pub mod traffic_lights;

use gpui::{Bounds, Pixels, WindowBounds, WindowOptions};

use self::traffic_lights::TrafficLightsHook;

/// Construct window options that enable the unified toolbar styling and apply the provided
/// traffic light configuration.
pub fn unified_window_options(
    bounds: Bounds<Pixels>,
    traffic_lights: &TrafficLightsHook,
) -> WindowOptions {
    let mut options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        // Wayland app_id (=> Hyprland `class`) and X11 WM_CLASS. Without it
        // the client maps with an empty class/title, so `hyprctl clients` and
        // WM rules cannot match the app (T037 window residual). The title is
        // intentionally left unset: the product draws its own "chronos-fm"
        // label in the unified toolbar (T037 visual spec §2.2).
        app_id: Some("chronos-fm".to_string()),
        ..Default::default()
    };

    traffic_lights.apply(&mut options);
    options
}
