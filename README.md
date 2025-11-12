# godot-ksni

A Godot 4 GDExtension that provides system tray icon functionality for Linux desktop environments using the StatusNotifierItem specification.

## Overview

godot-ksni wraps the [iovxw/ksni](https://github.com/iovxw/ksni) Rust library to bring native system tray support to Godot 4 projects on Linux. It exposes a `TrayIcon` node that can be used to create tray icons with menus, checkboxes, radio buttons, submenus, and custom icons.

![godot-ksni Tray Screenshot](assets/tray_screenshot.webp)

## Features

- System tray icon with custom images or system icon names
- Menu items with icons, enabled/disabled states, and visibility control
- Checkmark items that can be toggled
- Radio button groups for mutually exclusive options
- Submenus for organizing complex menus
- Signals for handling user interactions
- Support for Godot's resource system (works with exported games)
- Thread-safe state management

## Platform Support

This library currently supports Linux desktop environments that implement the StatusNotifierItem specification, including:

- KDE Plasma
- GNOME (with AppIndicator extension)
- COSMIC
- Other freedesktop-compliant environments

## Installation

### Method 1: As a Standalone GDExtension

Use this method if you want to add godot-ksni as a separate GDExtension to your Godot project.

1. Add godot-ksni as a submodule or clone to your project:

```bash
git submodule add https://github.com/yuna0x0/godot-ksni.git
git submodule update --init --recursive

# Or clone directly
git clone https://github.com/yuna0x0/godot-ksni.git
```

2. Build the library with default features (includes `gdextension` feature):

```bash
cd godot-ksni

# Debug build
cargo build

# Release build
cargo build --release
```

3. Assuming your project structure is like this:
```
.
├── godot
│   ├── GodotKsni.gdextension
│   ├── project.godot
│   └── ...
└── godot-ksni
    ├── Cargo.lock
    ├── Cargo.toml
    ├── src
    └── ...
```

4. Create a `GodotKsni.gdextension` file in your Godot project directory (e.g., `godot/GodotKsni.gdextension`) with the following content:
```gdextension
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.5
reloadable = true

[libraries]
linux.debug.x86_64 = "res://../godot-ksni/target/debug/libgodot_ksni.so"
linux.release.x86_64 = "res://../godot-ksni/target/release/libgodot_ksni.so"
```

5. The `TrayIcon` node will be available in your Godot project.

### Method 2: As a Rust Dependency (for GDExtension developers)

Use this method if you're building your own Rust GDExtension and want to include godot-ksni's functionality within it.

1. Add godot-ksni as a dependency with default features disabled:

```bash
cargo add godot-ksni --no-default-features
```

**Important**: You must disable default features with `--no-default-features` to prevent duplicate `gdext_rust_init` symbols. The `gdextension` feature (enabled by default) is only needed when building godot-ksni as its own standalone GDExtension (Method 1).

2. In your `lib.rs`, re-export the `TrayIcon` to ensure it gets linked:

```rust
use godot::prelude::*;

// Re-export TrayIcon so it's registered with Godot
pub use godot_ksni::TrayIcon;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}
```

3. The `TrayIcon` node will be automatically registered when your extension loads.

## Quick Start

```gdscript
extends Node

var tray_icon: TrayIcon

func _ready():
    # Create the TrayIcon node
    tray_icon = TrayIcon.new()
    add_child(tray_icon)

    # Configure the tray
    tray_icon.set_tray_id("my_application")
    tray_icon.set_title("My Application")
    tray_icon.set_icon_from_path("res://icon.svg")
    tray_icon.set_tooltip("My App", "Running in background", "")

    # Build menu
    tray_icon.add_menu_item("show", "Show Window", "", true, true)
    tray_icon.add_separator()
    tray_icon.add_menu_item("quit", "Quit", "application-exit", true, true)

    # Connect signals
    tray_icon.menu_activated.connect(_on_menu_activated)

    # Spawn the tray icon
    if tray_icon.spawn_tray():
        print("Tray icon created successfully")

func _on_menu_activated(id: String):
    match id:
        "show":
            get_window().visible = true
        "quit":
            get_tree().quit()
```

### Setting Icons

```gdscript
# Method 1: System icon (simplest, but limited selection)
tray_icon.set_icon_name("application-x-executable")

# Method 2: From resource path (recommended for custom icons)
if tray_icon.set_icon_from_path("res://icon.svg"):
    print("Icon loaded")

# Method 3: From texture resource
var texture = load("res://icon.svg")
if texture:
    tray_icon.set_icon_from_texture(texture)

# Method 4: From image (for image manipulation)
var texture = load("res://icon.svg")
if texture:
    var image = texture.get_image()
    tray_icon.set_icon_from_image(image)
```

### Building Complex Menus

```gdscript
func build_menu():
    tray_icon.clear_menu()

    # Standard items
    tray_icon.add_menu_item("open", "Open", "", true, true)
    tray_icon.add_separator()

    # Checkmark
    tray_icon.add_checkmark_item("autostart", "Start on Boot", "", false, true, true)
    tray_icon.add_separator()

    # Radio group
    tray_icon.add_radio_group("theme", 0)
    tray_icon.add_radio_option("theme", "light", "Light", "", true, true)
    tray_icon.add_radio_option("theme", "dark", "Dark", "", true, true)
    tray_icon.add_separator()

    # Submenu
    tray_icon.begin_submenu("Settings", "preferences-system", true, true)
    tray_icon.add_submenu_item("Settings", "prefs", "Preferences", "", true, true)
    tray_icon.add_submenu_checkmark("Settings", "notify", "Notifications", "", true, true, true)
    tray_icon.add_separator()

    tray_icon.add_menu_item("quit", "Quit", "application-exit", true, true)
```

## Examples

The `examples/` directory contains the following examples:
- `tray_example.gd` - Example demonstrating all features (menu items, checkmarks, radio groups, submenus)

## Building from Source

### Requirements

- Rust and Cargo
- Godot 4.5 or later
- Linux development headers (dbus, etc.)

### Build Steps

```bash
# Clone the repository (if not already done)
git clone https://github.com/yuna0x0/godot-ksni.git
cd godot-ksni

# Debug build
cargo build

# Release build (recommended for production)
cargo build --release
```

The compiled library will be in `target/debug/libgodot_ksni.so` or `target/release/libgodot_ksni.so`.

## Troubleshooting

### Tray icon not appearing

- Ensure your desktop environment supports StatusNotifierItem (SNI)
- On GNOME, install the AppIndicator extension
- Check console output for error messages

### Icon not loading

- Verify the icon path is correct (use `res://` for Godot resources)
- Ensure the image format is supported by Godot (PNG, SVG, etc.)
- Fall back to system icon names if custom icons fail

## License

This project is dual-licensed under either:

- MIT License
- Apache License 2.0

at your option.
