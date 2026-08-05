//! Reusable GPUI building blocks for chronos-fm.
//!
//! `chronos-fm-ui` hosts the toolkit-level pieces — theme, assets, window chrome, and
//! presentational components — deliberately kept free of app- or page-specific
//! wiring so they can be reused (and eventually published) independently of the
//! chronos-fm binary. It depends only on `chronos-fm-core` and `chronos-fm-models`; the app
//! shell that orchestrates pages lives in the `chronos-fm` binary crate.

/// Embedded crate-local assets (icons, fonts) exposed as a GPUI `AssetSource`.
pub mod assets;
/// Presentational UI components built on GPUI.
pub mod components;
/// Live removable-media device list (GPUI global).
pub mod devices_store;
/// Structural UI patterns (elevated_card, section_header) ported from ChronOS.
pub mod patterns;
/// Theme tokens (colors) shared across components.
pub mod theme;
/// Window construction helpers, including unified toolbar window options.
pub mod window;

pub use components::file_list;
