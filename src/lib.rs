//! # godot-ksni
//!
//! A Godot 4 GDExtension that provides system tray icon functionality for Linux desktop environments
//! using the StatusNotifierItem (SNI) specification via the [ksni](https://crates.io/crates/ksni) library.
//!
//! ## Overview
//!
//! This library exposes a `TrayIcon` node that can be used in Godot projects to create system tray icons
//! with menus, tooltips, and custom icons. It supports standard menu items, checkboxes, radio buttons,
//! submenus, and separators.
//!
//! ## Usage
//!
//! ### Method 1: As a Standalone GDExtension
//!
//! Use this method if you want to add godot-ksni as a separate GDExtension to your Godot project.
//!
//! 1. Add godot-ksni as a git submodule or clone to your project
//!    ```bash
//!    git submodule add https://github.com/yuna0x0/godot-ksni.git
//!    git submodule update --init --recursive
//!
//!    # Or clone directly
//!    git clone https://github.com/yuna0x0/godot-ksni.git
//!    ```
//!
//! 2. Build the library with default features (includes `gdextension` feature):
//!    ```bash
//!    cd godot-ksni
//!
//!    # For debug build
//!    cargo build
//!
//!    # For release build
//!    cargo build --release
//!    ```
//!
//! 3. Assuming your project structure is like this:
//!    ```text
//!    .
//!    ├── godot
//!    │   ├── GodotKsni.gdextension
//!    │   ├── project.godot
//!    │   └── ...
//!    └── godot-ksni
//!        ├── Cargo.toml
//!        ├── src
//!        ├── target
//!        └── ...
//!    ```
//!
//! 4. Create a `GodotKsni.gdextension` file in your Godot project directory:
//!    ```gdextension
//!    [configuration]
//!    entry_symbol = "gdext_rust_init"
//!    compatibility_minimum = 4.5
//!    reloadable = true
//!
//!    [libraries]
//!    linux.debug.x86_64 = "res://../godot-ksni/target/debug/libgodot_ksni.so"
//!    linux.release.x86_64 = "res://../godot-ksni/target/release/libgodot_ksni.so"
//!    ```
//!
//! 5. The `TrayIcon` node will be available in your Godot project
//!
//! ### Method 2: As a Rust Dependency (for GDExtension developers)
//!
//! Use this method if you're building your own Rust GDExtension and want to include
//! godot-ksni's functionality within it.
//!
//! 1. Add godot-ksni as a dependency with default features disabled:
//!    ```bash
//!    cargo add godot-ksni --no-default-features
//!    ```
//!
//!    **Important**: You must disable default features to prevent duplicate `gdext_rust_init`
//!    symbols. The `gdextension` feature is only needed when building as a standalone library.
//!
//! 2. In your `lib.rs`, re-export `TrayIcon` to ensure it gets linked:
//!    ```rust,ignore
//!    use godot::prelude::*;
//!
//!    // Re-export TrayIcon so it's registered with Godot
//!    pub use godot_ksni::TrayIcon;
//!
//!    struct MyExtension;
//!
//!    #[gdextension]
//!    unsafe impl ExtensionLibrary for MyExtension {}
//!    ```
//!
//! 3. The `TrayIcon` node will be automatically registered when your extension loads
//!
//! ## Example
//!
//! ```gdscript
//! extends Node
//!
//! var tray_icon: TrayIcon
//!
//! func _ready():
//!     tray_icon = TrayIcon.new()
//!     add_child(tray_icon)
//!
//!     tray_icon.set_tray_id("my_app")
//!     tray_icon.set_title("My Application")
//!     tray_icon.set_icon_from_path("res://icon.svg")
//!
//!     tray_icon.add_menu_item("quit", "Quit", "application-exit", true, true)
//!
//!     tray_icon.menu_activated.connect(_on_menu_activated)
//!     tray_icon.spawn_tray()
//!
//! func _on_menu_activated(id: String):
//!     if id == "quit":
//!         get_tree().quit()
//! ```

// Module declarations
pub mod godot;
pub mod menu;
pub mod tray;

// Public re-exports
pub use godot::TrayIcon;
pub use menu::{MenuItemData, RadioItemData};
pub use tray::{KsniTray, TrayEvent, TrayState};

// Conditional GDExtension entry point
#[cfg(feature = "gdextension")]
mod gdextension {
    use godot::prelude::*;

    struct GodotKsniExtension;

    #[gdextension]
    unsafe impl ExtensionLibrary for GodotKsniExtension {}
}
