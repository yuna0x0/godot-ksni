//! Tray core functionality.
//!
//! This module contains the core tray icon functionality, including state management,
//! event handling, and the bridge to the KSNI library.

pub mod event;
pub mod ksni_impl;
pub mod state;

pub use event::TrayEvent;
pub use ksni_impl::KsniTray;
pub use state::TrayState;
