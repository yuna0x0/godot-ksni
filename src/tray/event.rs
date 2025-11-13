//! Internal events emitted by the tray icon.
//!
//! These events are used internally to communicate between the tray icon
//! and the Godot node, and are converted to Godot signals.

/// Internal events emitted by the tray icon.
///
/// These events are used internally to communicate between the tray icon
/// and the Godot node, and are converted to Godot signals.
pub enum TrayEvent {
    /// A standard menu item was activated.
    MenuActivated(String),
    /// A checkmark menu item was toggled.
    CheckmarkToggled(String, bool),
    /// A radio button option was selected.
    RadioSelected(String, usize, String),
}
