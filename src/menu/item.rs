//! Menu item data structures.
//!
//! This module defines the various types of menu items that can be added to the tray menu,
//! including standard items, checkmarks, radio groups, submenus, and separators.

/// Represents different types of menu items that can be added to the tray menu.
///
/// This enum defines all the possible menu item types supported by the tray icon,
/// including standard items, checkmarks, radio groups, submenus, and separators.
#[derive(Clone, Debug)]
pub enum MenuItemData {
    /// A standard clickable menu item.
    Standard {
        /// Unique identifier for the menu item.
        id: String,
        /// Display text for the menu item.
        label: String,
        /// Icon name from the freedesktop icon theme.
        icon_name: String,
        /// Whether the item can be clicked.
        enabled: bool,
        /// Whether the item is visible in the menu.
        visible: bool,
    },
    /// A menu item with a checkmark that can be toggled on/off.
    Checkmark {
        /// Unique identifier for the checkmark item.
        id: String,
        /// Display text for the checkmark item.
        label: String,
        /// Icon name from the freedesktop icon theme.
        icon_name: String,
        /// Whether the item can be clicked.
        enabled: bool,
        /// Whether the item is visible in the menu.
        visible: bool,
        /// Current checked state.
        checked: bool,
    },
    /// A group of mutually exclusive radio button options.
    RadioGroup {
        /// Unique identifier for the radio group.
        id: String,
        /// Index of the currently selected option.
        selected: usize,
        /// List of radio button options in this group.
        options: Vec<RadioItemData>,
    },
    /// A submenu that contains other menu items.
    SubMenu {
        /// Display text for the submenu.
        label: String,
        /// Icon name from the freedesktop icon theme.
        icon_name: String,
        /// Whether the submenu can be opened.
        enabled: bool,
        /// Whether the submenu is visible in the menu.
        visible: bool,
        /// List of menu items contained in this submenu.
        submenu: Vec<MenuItemData>,
    },
    /// A visual separator line in the menu.
    Separator,
}

/// Data for a single radio button option within a radio group.
///
/// Each radio option has its own identifier, label, and visual properties.
#[derive(Clone, Debug)]
pub struct RadioItemData {
    /// Unique identifier for this radio option.
    pub id: String,
    /// Display text for this radio option.
    pub label: String,
    /// Icon name from the freedesktop icon theme.
    pub icon_name: String,
    /// Whether this option can be selected.
    pub enabled: bool,
    /// Whether this option is visible in the menu.
    pub visible: bool,
}
