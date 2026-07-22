//! Baobab is [`boa_cli`] in [`egui`]
//!
//! The basic usage is for a quick and easy js playground (for example, bind a keyboard shortcut to `baobab`).
//!
//! ## Usage
//!
//! ```sh
//! cargo install baobab
//! baobab
//! ```

#![warn(clippy::all, rust_2018_idioms)]
#![deny(
    missing_docs,
    clippy::all,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cargo,
    clippy::unwrap_used,
    clippy::expect_used
)]
#![allow(clippy::multiple_crate_versions)]

mod app;
pub use app::BaobabApp;
pub use app::SendType;
