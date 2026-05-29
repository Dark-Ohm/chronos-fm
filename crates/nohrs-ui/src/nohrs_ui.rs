//! Reusable GPUI building blocks for nohrs.
//!
//! `nohrs-ui` hosts the toolkit-level pieces — theme, assets, window chrome, and
//! presentational components — deliberately kept free of app- or page-specific
//! wiring so they can be reused (and eventually published) independently of the
//! nohrs binary. It depends only on `nohrs-core` and `nohrs-models`; the app
//! shell that orchestrates pages lives in the `nohrs` binary crate.

pub mod assets;
pub mod components;
pub mod theme;
pub mod window;

pub use components::file_list;
