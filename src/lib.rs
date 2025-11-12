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
//! 1. Add godot-ksni as a git submodule to your project
//! 2. Build the library with default features (includes `gdextension` feature):
//!    ```bash
//!    cd godot-ksni && cargo build --release
//!    ```
//! 3. Create a `GodotKsni.gdextension` file in your Godot project directory:
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
//! 4. The `TrayIcon` node will be available in your Godot project
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
//!     tray_icon.set_icon_from_path("res://icon.png")
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

use godot::classes::{Image, ResourceLoader, Texture2D};
use godot::prelude::*;
use ksni::blocking::TrayMethods;
use ksni::menu::*;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};

enum TrayEvent {
    MenuActivated(String),
    CheckmarkToggled(String, bool),
    RadioSelected(String, usize, String),
}

struct TrayState {
    icon_name: String,
    icon_theme_path: String,
    icon_pixmap: Vec<ksni::Icon>,
    title: String,
    tooltip_title: String,
    tooltip_subtitle: String,
    tooltip_icon_name: String,
    tray_id: String,
    menu: Vec<MenuItemData>,
    event_sender: Option<Sender<TrayEvent>>,
}

#[derive(Clone, Debug)]
enum MenuItemData {
    Standard {
        id: String,
        label: String,
        icon_name: String,
        enabled: bool,
        visible: bool,
    },
    Checkmark {
        id: String,
        label: String,
        icon_name: String,
        enabled: bool,
        visible: bool,
        checked: bool,
    },
    RadioGroup {
        id: String,
        selected: usize,
        options: Vec<RadioItemData>,
    },
    SubMenu {
        label: String,
        icon_name: String,
        enabled: bool,
        visible: bool,
        submenu: Vec<MenuItemData>,
    },
    Separator,
}

#[derive(Clone, Debug)]
struct RadioItemData {
    id: String,
    label: String,
    icon_name: String,
    enabled: bool,
    visible: bool,
}

impl TrayState {
    fn new(tray_id: String) -> Self {
        Self {
            icon_name: "application-x-executable".to_string(),
            icon_theme_path: String::new(),
            icon_pixmap: Vec::new(),
            title: "Tray Icon".to_string(),
            tooltip_title: String::new(),
            tooltip_subtitle: String::new(),
            tooltip_icon_name: String::new(),
            tray_id,
            menu: Vec::new(),
            event_sender: None,
        }
    }

    fn find_and_toggle_checkmark(&mut self, id: &str) -> Option<bool> {
        Self::find_and_toggle_checkmark_recursive(&mut self.menu, id)
    }

    fn find_and_toggle_checkmark_recursive(
        items: &mut Vec<MenuItemData>,
        id: &str,
    ) -> Option<bool> {
        for menu_item in items {
            match menu_item {
                MenuItemData::Checkmark {
                    id: item_id,
                    checked,
                    ..
                } => {
                    if item_id == id {
                        *checked = !*checked;
                        return Some(*checked);
                    }
                }
                MenuItemData::SubMenu { submenu, .. } => {
                    if let Some(result) = Self::find_and_toggle_checkmark_recursive(submenu, id) {
                        return Some(result);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn find_and_select_radio(&mut self, group_id: &str, index: usize) -> Option<String> {
        Self::find_and_select_radio_recursive(&mut self.menu, group_id, index)
    }

    fn find_and_select_radio_recursive(
        items: &mut Vec<MenuItemData>,
        group_id: &str,
        index: usize,
    ) -> Option<String> {
        for menu_item in items {
            match menu_item {
                MenuItemData::RadioGroup {
                    id,
                    selected,
                    options,
                } => {
                    if id == group_id && index < options.len() {
                        *selected = index;
                        return Some(options[index].id.clone());
                    }
                }
                MenuItemData::SubMenu { submenu, .. } => {
                    if let Some(result) =
                        Self::find_and_select_radio_recursive(submenu, group_id, index)
                    {
                        return Some(result);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn build_menu_items(&self) -> Vec<MenuItem<KsniTray>> {
        self.menu
            .iter()
            .map(|item| self.build_menu_item(item))
            .collect()
    }

    fn build_menu_item(&self, item: &MenuItemData) -> MenuItem<KsniTray> {
        match item {
            MenuItemData::Standard {
                id,
                label,
                icon_name,
                enabled,
                visible,
            } => {
                let id_clone = id.clone();
                let sender = self.event_sender.clone();
                StandardItem {
                    label: label.clone(),
                    icon_name: icon_name.clone(),
                    enabled: *enabled,
                    visible: *visible,
                    activate: Box::new(move |_this: &mut KsniTray| {
                        if let Some(ref tx) = sender {
                            let _ = tx.send(TrayEvent::MenuActivated(id_clone.clone()));
                        }
                    }),
                    ..Default::default()
                }
                .into()
            }
            MenuItemData::Checkmark {
                id,
                label,
                icon_name,
                enabled,
                visible,
                checked,
            } => {
                let id_clone = id.clone();
                let sender = self.event_sender.clone();
                CheckmarkItem {
                    label: label.clone(),
                    icon_name: icon_name.clone(),
                    enabled: *enabled,
                    visible: *visible,
                    checked: *checked,
                    activate: Box::new(move |this: &mut KsniTray| {
                        let new_checked = {
                            let mut state = this.state.lock().unwrap();
                            state.find_and_toggle_checkmark(&id_clone)
                        };

                        if let (Some(tx), Some(checked)) = (&sender, new_checked) {
                            let _ = tx.send(TrayEvent::CheckmarkToggled(id_clone.clone(), checked));
                        }
                    }),
                    ..Default::default()
                }
                .into()
            }
            MenuItemData::RadioGroup {
                id,
                selected,
                options,
            } => {
                let id_clone = id.clone();
                let sender = self.event_sender.clone();
                RadioGroup {
                    selected: *selected,
                    select: Box::new(move |this: &mut KsniTray, index| {
                        let option_id = {
                            let mut state = this.state.lock().unwrap();
                            state.find_and_select_radio(&id_clone, index)
                        };

                        if let (Some(tx), Some(opt_id)) = (&sender, option_id) {
                            let _ =
                                tx.send(TrayEvent::RadioSelected(id_clone.clone(), index, opt_id));
                        }
                    }),
                    options: options
                        .iter()
                        .map(|opt| RadioItem {
                            label: opt.label.clone(),
                            icon_name: opt.icon_name.clone(),
                            enabled: opt.enabled,
                            visible: opt.visible,
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                }
                .into()
            }
            MenuItemData::SubMenu {
                label,
                icon_name,
                enabled,
                visible,
                submenu,
            } => SubMenu {
                label: label.clone(),
                icon_name: icon_name.clone(),
                enabled: *enabled,
                visible: *visible,
                submenu: submenu
                    .iter()
                    .map(|item| self.build_menu_item(item))
                    .collect(),
                ..Default::default()
            }
            .into(),
            MenuItemData::Separator => MenuItem::Separator,
        }
    }
}

struct KsniTray {
    state: Arc<Mutex<TrayState>>,
}

impl ksni::Tray for KsniTray {
    fn id(&self) -> String {
        let state = self.state.lock().unwrap();
        state.tray_id.clone()
    }

    fn icon_name(&self) -> String {
        let state = self.state.lock().unwrap();
        state.icon_name.clone()
    }

    fn icon_theme_path(&self) -> String {
        let state = self.state.lock().unwrap();
        state.icon_theme_path.clone()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let state = self.state.lock().unwrap();
        state.icon_pixmap.clone()
    }

    fn title(&self) -> String {
        let state = self.state.lock().unwrap();
        state.title.clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let state = self.state.lock().unwrap();
        ksni::ToolTip {
            icon_name: state.tooltip_icon_name.clone(),
            icon_pixmap: vec![],
            title: state.tooltip_title.clone(),
            description: state.tooltip_subtitle.clone(),
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let state = self.state.lock().unwrap();
        state.build_menu_items()
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
/// A Godot node that provides system tray icon functionality for Linux.
///
/// `TrayIcon` creates and manages a system tray icon using the StatusNotifierItem specification.
/// It supports custom icons, menus with various item types, and signals for user interactions.
///
/// # Signals
///
/// - `menu_activated(id: String)` - Emitted when a standard menu item is clicked
/// - `checkmark_toggled(id: String, checked: bool)` - Emitted when a checkmark item is toggled
/// - `radio_selected(group_id: String, index: int, option_id: String)` - Emitted when a radio option is selected
///
/// # Example
///
/// ```gdscript
/// var tray = TrayIcon.new()
/// add_child(tray)
/// tray.set_tray_id("my_app")
/// tray.set_icon_from_path("res://icon.png")
/// tray.menu_activated.connect(_on_menu_activated)
/// tray.spawn_tray()
/// ```
pub struct TrayIcon {
    base: Base<Node>,
    handle: Option<ksni::blocking::Handle<KsniTray>>,
    state: Arc<Mutex<TrayState>>,
    event_receiver: Option<std::sync::mpsc::Receiver<TrayEvent>>,
}

#[godot_api]
impl INode for TrayIcon {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            handle: None,
            state: Arc::new(Mutex::new(TrayState::new("godot_tray_icon".to_string()))),
            event_receiver: None,
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_process(true);
    }

    fn process(&mut self, _delta: f64) {
        let mut events = Vec::new();
        if let Some(ref rx) = self.event_receiver {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }

        for event in events {
            match event {
                TrayEvent::MenuActivated(id) => {
                    self.base_mut()
                        .emit_signal("menu_activated", &[Variant::from(id)]);
                }
                TrayEvent::CheckmarkToggled(id, checked) => {
                    self.base_mut().emit_signal(
                        "checkmark_toggled",
                        &[Variant::from(id), Variant::from(checked)],
                    );
                    self.update_tray();
                }
                TrayEvent::RadioSelected(group_id, index, option_id) => {
                    self.base_mut().emit_signal(
                        "radio_selected",
                        &[
                            Variant::from(group_id),
                            Variant::from(index as i64),
                            Variant::from(option_id),
                        ],
                    );
                    self.update_tray();
                }
            }
        }
    }
}

#[godot_api]
impl TrayIcon {
    #[signal]
    fn menu_activated(id: GString);

    #[signal]
    fn checkmark_toggled(id: GString, checked: bool);

    #[signal]
    fn radio_selected(group_id: GString, index: i64, option_id: GString);

    /// Spawns the system tray icon.
    ///
    /// This method must be called after configuring the tray icon to make it visible in the system tray.
    /// It should only be called once. Subsequent calls will be ignored and return false.
    ///
    /// # Returns
    ///
    /// Returns `true` if the tray was successfully spawned, `false` if it was already spawned or if an error occurred.
    ///
    /// # Example
    ///
    /// ```gdscript
    /// if tray_icon.spawn_tray():
    ///     print("Tray icon created successfully")
    /// else:
    ///     print("Failed to create tray icon")
    /// ```
    #[func]
    fn spawn_tray(&mut self) -> bool {
        if self.handle.is_some() {
            godot_warn!("Tray already spawned");
            return false;
        }

        let (tx, rx) = channel();
        self.event_receiver = Some(rx);

        {
            let mut state = self.state.lock().unwrap();
            state.event_sender = Some(tx);
        }

        let state_arc = self.state.clone();
        let tray = KsniTray { state: state_arc };

        match tray.spawn() {
            Ok(handle) => {
                self.handle = Some(handle);
                true
            }
            Err(e) => {
                godot_error!("Failed to spawn tray: {}", e);
                false
            }
        }
    }

    /// Updates the tray icon to reflect changes made to its state.
    ///
    /// This method is automatically called when checkmarks or radio buttons are toggled.
    /// Manual calls are rarely needed.
    #[func]
    fn update_tray(&mut self) {
        if let Some(handle) = &self.handle {
            handle.update(|_tray: &mut KsniTray| {});
        }
    }

    /// Sets the unique identifier for this tray icon.
    ///
    /// The ID is used by the system to identify this tray icon. It should be unique per application.
    ///
    /// # Parameters
    ///
    /// - `tray_id` - A unique identifier string (e.g., "com.example.myapp")
    #[func]
    fn set_tray_id(&mut self, tray_id: GString) {
        let mut state = self.state.lock().unwrap();
        state.tray_id = tray_id.to_string();
    }

    /// Sets the tray icon using a system icon name.
    ///
    /// Uses the freedesktop icon naming specification. Common names include:
    /// - "application-x-executable"
    /// - "applications-games"
    /// - "help-about"
    ///
    /// # Parameters
    ///
    /// - `icon_name` - The name of the system icon to use
    #[func]
    fn set_icon_name(&mut self, icon_name: GString) {
        let mut state = self.state.lock().unwrap();
        state.icon_name = icon_name.to_string();
    }

    /// Sets the path to search for icon themes.
    ///
    /// # Parameters
    ///
    /// - `path` - The filesystem path to the icon theme directory
    #[func]
    fn set_icon_theme_path(&mut self, path: GString) {
        let mut state = self.state.lock().unwrap();
        state.icon_theme_path = path.to_string();
    }

    /// Sets the tray icon from a Godot Image resource.
    ///
    /// # Parameters
    /// * `image` - A Godot Image resource
    ///
    /// # Returns
    /// `true` if the icon was set successfully, `false` otherwise
    ///
    /// # Example (GDScript)
    /// ```gdscript
    /// var texture = load("res://icon.png")
    /// var image = texture.get_image()
    /// tray_icon.set_icon_from_image(image)
    /// ```
    #[func]
    fn set_icon_from_image(&mut self, image: Gd<Image>) -> bool {
        // Get image dimensions
        let width = image.get_width();
        let height = image.get_height();

        if width <= 0 || height <= 0 {
            godot_error!("Invalid image dimensions: {}x{}", width, height);
            return false;
        }

        // Convert to RGBA8 if needed
        let mut img = image.duplicate().unwrap().cast::<Image>();
        img.convert(godot::classes::image::Format::RGBA8);

        // Get pixel data
        let data = img.get_data();
        let bytes: Vec<u8> = data.to_vec();

        if bytes.len() != (width * height * 4) as usize {
            godot_error!(
                "Image data size mismatch: expected {}, got {}",
                width * height * 4,
                bytes.len()
            );
            return false;
        }

        // Convert RGBA to ARGB for ksni
        let mut argb_data = bytes.clone();
        for pixel in argb_data.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }

        let mut state = self.state.lock().unwrap();
        state.icon_pixmap = vec![ksni::Icon {
            width,
            height,
            data: argb_data,
        }];
        state.icon_name = String::new();
        true
    }

    /// Sets the tray icon from a Godot Texture2D resource.
    /// This is the recommended method for most use cases.
    ///
    /// Works with exported games because it uses Godot's resource system.
    ///
    /// # Parameters
    /// * `texture` - A Godot Texture2D resource (CompressedTexture2D, ImageTexture, etc.)
    ///
    /// # Returns
    /// `true` if the icon was set successfully, `false` otherwise
    ///
    /// # Example (GDScript)
    /// ```gdscript
    /// var texture = load("res://icon.png")
    /// tray_icon.set_icon_from_texture(texture)
    /// ```
    #[func]
    fn set_icon_from_texture(&mut self, texture: Gd<Texture2D>) -> bool {
        let image = texture.get_image();

        if image.is_none() {
            godot_error!("Failed to get image from texture");
            return false;
        }

        self.set_icon_from_image(image.unwrap())
    }

    /// Sets the tray icon by loading a texture from a Godot resource path.
    /// This is a convenience wrapper around set_icon_from_texture().
    ///
    /// Works with exported games because it uses ResourceLoader.
    ///
    /// # Parameters
    /// * `path` - A Godot resource path (e.g., "res://icon.png")
    ///
    /// # Returns
    /// `true` if the icon was loaded and set successfully, `false` otherwise
    ///
    /// # Example (GDScript)
    /// ```gdscript
    /// tray_icon.set_icon_from_path("res://icon.png")
    /// ```
    #[func]
    fn set_icon_from_path(&mut self, path: GString) -> bool {
        let mut loader = ResourceLoader::singleton();
        let resource = loader.load(&path);

        if resource.is_none() {
            godot_error!("Failed to load resource from path: {}", path);
            return false;
        }

        let texture = resource.unwrap().try_cast::<Texture2D>();
        if texture.is_err() {
            godot_error!("Resource is not a Texture2D: {}", path);
            return false;
        }

        self.set_icon_from_texture(texture.unwrap())
    }

    #[func]
    fn set_icon_from_data(&mut self, width: i32, height: i32, data: PackedByteArray) -> bool {
        let bytes: Vec<u8> = data.to_vec();

        if bytes.len() != (width * height * 4) as usize {
            godot_error!("Invalid icon data size");
            return false;
        }

        let mut argb_data = bytes.clone();
        for pixel in argb_data.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }

        let mut state = self.state.lock().unwrap();
        state.icon_pixmap = vec![ksni::Icon {
            width,
            height,
            data: argb_data,
        }];
        state.icon_name = String::new();
        true
    }

    #[func]
    fn clear_icon_pixmap(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.icon_pixmap.clear();
    }

    /// Sets the title text displayed next to the tray icon.
    ///
    /// # Parameters
    ///
    /// - `title` - The title text to display
    #[func]
    fn set_title(&mut self, title: GString) {
        let mut state = self.state.lock().unwrap();
        state.title = title.to_string();
    }

    /// Sets the tooltip displayed when hovering over the tray icon.
    ///
    /// # Parameters
    ///
    /// - `title` - The main tooltip text
    /// - `subtitle` - Additional tooltip text displayed below the title
    /// - `icon_name` - System icon name to display in the tooltip
    #[func]
    fn set_tooltip(&mut self, title: GString, subtitle: GString, icon_name: GString) {
        let mut state = self.state.lock().unwrap();
        state.tooltip_title = title.to_string();
        state.tooltip_subtitle = subtitle.to_string();
        state.tooltip_icon_name = icon_name.to_string();
    }

    /// Clears all menu items from the tray menu.
    ///
    /// This is useful when rebuilding the menu from scratch.
    #[func]
    fn clear_menu(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.menu.clear();
    }

    /// Adds a standard clickable menu item.
    ///
    /// When clicked, emits the `menu_activated` signal with the item's ID.
    ///
    /// # Parameters
    ///
    /// - `id` - Unique identifier for this menu item
    /// - `label` - Text displayed in the menu
    /// - `icon_name` - System icon name (empty string for no icon)
    /// - `enabled` - Whether the item can be clicked
    /// - `visible` - Whether the item is visible
    #[func]
    fn add_menu_item(
        &mut self,
        id: GString,
        label: GString,
        icon_name: GString,
        enabled: bool,
        visible: bool,
    ) {
        let mut state = self.state.lock().unwrap();
        state.menu.push(MenuItemData::Standard {
            id: id.to_string(),
            label: label.to_string(),
            icon_name: icon_name.to_string(),
            enabled,
            visible,
        });
    }

    /// Adds a menu item with a checkmark that can be toggled.
    ///
    /// When toggled, emits the `checkmark_toggled` signal with the item's ID and new state.
    ///
    /// # Parameters
    ///
    /// - `id` - Unique identifier for this checkmark item
    /// - `label` - Text displayed in the menu
    /// - `icon_name` - System icon name (empty string for no icon)
    /// - `checked` - Initial checked state
    /// - `enabled` - Whether the item can be clicked
    /// - `visible` - Whether the item is visible
    #[func]
    fn add_checkmark_item(
        &mut self,
        id: GString,
        label: GString,
        icon_name: GString,
        checked: bool,
        enabled: bool,
        visible: bool,
    ) {
        let mut state = self.state.lock().unwrap();
        state.menu.push(MenuItemData::Checkmark {
            id: id.to_string(),
            label: label.to_string(),
            icon_name: icon_name.to_string(),
            enabled,
            visible,
            checked,
        });
    }

    /// Creates a new radio button group.
    ///
    /// Radio options must be added to this group using `add_radio_option`.
    /// Only one option in a group can be selected at a time.
    ///
    /// # Parameters
    ///
    /// - `id` - Unique identifier for this radio group
    /// - `selected` - Index of the initially selected option (0-based)
    #[func]
    fn add_radio_group(&mut self, id: GString, selected: i64) {
        let mut state = self.state.lock().unwrap();
        state.menu.push(MenuItemData::RadioGroup {
            id: id.to_string(),
            selected: selected as usize,
            options: Vec::new(),
        });
    }

    /// Adds a radio button option to an existing radio group.
    ///
    /// When selected, emits the `radio_selected` signal with the group ID, option index, and option ID.
    ///
    /// # Parameters
    ///
    /// - `group_id` - ID of the radio group to add this option to
    /// - `option_id` - Unique identifier for this option
    /// - `label` - Text displayed in the menu
    /// - `icon_name` - System icon name (empty string for no icon)
    /// - `enabled` - Whether the option can be selected
    /// - `visible` - Whether the option is visible
    ///
    /// # Returns
    ///
    /// Returns `true` if the option was added successfully, `false` if the group was not found.
    #[func]
    fn add_radio_option(
        &mut self,
        group_id: GString,
        option_id: GString,
        label: GString,
        icon_name: GString,
        enabled: bool,
        visible: bool,
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        let group_id_str = group_id.to_string();

        for item in &mut state.menu {
            if let MenuItemData::RadioGroup { id, options, .. } = item
                && id == &group_id_str
            {
                options.push(RadioItemData {
                    id: option_id.to_string(),
                    label: label.to_string(),
                    icon_name: icon_name.to_string(),
                    enabled,
                    visible,
                });
                return true;
            }
        }
        false
    }

    /// Adds a visual separator line to the menu.
    #[func]
    fn add_separator(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.menu.push(MenuItemData::Separator);
    }

    /// Creates a submenu that can contain other menu items.
    ///
    /// After calling this, use `add_submenu_item`, `add_submenu_checkmark`, and `add_submenu_separator`
    /// to add items to the submenu.
    ///
    /// # Parameters
    ///
    /// - `label` - Text displayed for the submenu
    /// - `icon_name` - System icon name (empty string for no icon)
    /// - `enabled` - Whether the submenu can be opened
    /// - `visible` - Whether the submenu is visible
    #[func]
    fn begin_submenu(&mut self, label: GString, icon_name: GString, enabled: bool, visible: bool) {
        let mut state = self.state.lock().unwrap();
        state.menu.push(MenuItemData::SubMenu {
            label: label.to_string(),
            icon_name: icon_name.to_string(),
            enabled,
            visible,
            submenu: Vec::new(),
        });
    }

    /// Adds a standard menu item to an existing submenu.
    ///
    /// # Parameters
    ///
    /// - `submenu_label` - Label of the parent submenu
    /// - `id` - Unique identifier for this menu item
    /// - `label` - Text displayed in the submenu
    /// - `icon_name` - System icon name (empty string for no icon)
    /// - `enabled` - Whether the item can be clicked
    /// - `visible` - Whether the item is visible
    ///
    /// # Returns
    ///
    /// Returns `true` if the item was added successfully, `false` if the submenu was not found.
    #[func]
    fn add_submenu_item(
        &mut self,
        submenu_label: GString,
        id: GString,
        label: GString,
        icon_name: GString,
        enabled: bool,
        visible: bool,
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        let submenu_label_str = submenu_label.to_string();

        for item in &mut state.menu {
            if let MenuItemData::SubMenu {
                label: sub_label,
                submenu,
                ..
            } = item
                && sub_label == &submenu_label_str
            {
                submenu.push(MenuItemData::Standard {
                    id: id.to_string(),
                    label: label.to_string(),
                    icon_name: icon_name.to_string(),
                    enabled,
                    visible,
                });
                return true;
            }
        }
        false
    }

    /// Adds a checkmark item to an existing submenu.
    ///
    /// # Parameters
    ///
    /// - `submenu_label` - Label of the parent submenu
    /// - `id` - Unique identifier for this checkmark item
    /// - `label` - Text displayed in the submenu
    /// - `icon_name` - System icon name (empty string for no icon)
    /// - `checked` - Initial checked state
    /// - `enabled` - Whether the item can be clicked
    /// - `visible` - Whether the item is visible
    ///
    /// # Returns
    ///
    /// Returns `true` if the item was added successfully, `false` if the submenu was not found.
    #[func]
    fn add_submenu_checkmark(
        &mut self,
        submenu_label: GString,
        id: GString,
        label: GString,
        icon_name: GString,
        checked: bool,
        enabled: bool,
        visible: bool,
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        let submenu_label_str = submenu_label.to_string();

        for item in &mut state.menu {
            if let MenuItemData::SubMenu {
                label: sub_label,
                submenu,
                ..
            } = item
                && sub_label == &submenu_label_str
            {
                submenu.push(MenuItemData::Checkmark {
                    id: id.to_string(),
                    label: label.to_string(),
                    icon_name: icon_name.to_string(),
                    enabled,
                    visible,
                    checked,
                });
                return true;
            }
        }
        false
    }

    /// Adds a separator to an existing submenu.
    ///
    /// # Parameters
    ///
    /// - `submenu_label` - Label of the parent submenu
    ///
    /// # Returns
    ///
    /// Returns `true` if the separator was added successfully, `false` if the submenu was not found.
    #[func]
    fn add_submenu_separator(&mut self, submenu_label: GString) -> bool {
        let mut state = self.state.lock().unwrap();
        let submenu_label_str = submenu_label.to_string();

        for item in &mut state.menu {
            if let MenuItemData::SubMenu {
                label: sub_label,
                submenu,
                ..
            } = item
                && sub_label == &submenu_label_str
            {
                submenu.push(MenuItemData::Separator);
                return true;
            }
        }
        false
    }

    /// Programmatically sets the state of a checkmark item.
    ///
    /// # Parameters
    ///
    /// - `id` - ID of the checkmark item to modify
    /// - `checked` - New checked state
    ///
    /// # Returns
    ///
    /// Returns `true` if the checkmark was found and updated, `false` otherwise.
    #[func]
    fn set_checkmark_state(&mut self, id: GString, checked: bool) -> bool {
        let mut state = self.state.lock().unwrap();
        let id_str = id.to_string();

        for item in &mut state.menu {
            if let MenuItemData::Checkmark {
                id: item_id,
                checked: item_checked,
                ..
            } = item
                && item_id == &id_str
            {
                *item_checked = checked;
                return true;
            }
        }
        false
    }

    /// Programmatically selects a radio option in a radio group.
    ///
    /// # Parameters
    ///
    /// - `group_id` - ID of the radio group
    /// - `index` - Index of the option to select (0-based)
    ///
    /// # Returns
    ///
    /// Returns `true` if the group was found and the selection was updated, `false` otherwise.
    #[func]
    fn set_radio_selected(&mut self, group_id: GString, index: i64) -> bool {
        let mut state = self.state.lock().unwrap();
        let group_id_str = group_id.to_string();

        for item in &mut state.menu {
            if let MenuItemData::RadioGroup {
                id,
                selected,
                options,
            } = item
                && id == &group_id_str
                && (index as usize) < options.len()
            {
                *selected = index as usize;
                return true;
            }
        }
        false
    }
}

#[cfg(feature = "gdextension")]
mod gdextension {
    use super::*;

    struct GodotKsniExtension;

    #[gdextension]
    unsafe impl ExtensionLibrary for GodotKsniExtension {}
}
