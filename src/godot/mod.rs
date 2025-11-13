//! Godot integration.
//!
//! This module contains the Godot node implementation that exposes the tray icon
//! functionality to GDScript through the GDExtension API.

pub mod tray_icon;

pub use tray_icon::TrayIcon;
