//! Tray state management.
//!
//! This module contains the internal state of the tray icon and methods for
//! managing menu items, including finding and toggling checkmarks and radio buttons.

use crate::menu::item::MenuItemData;
use crate::tray::event::TrayEvent;
use crate::tray::ksni_impl::KsniTray;
use ksni::menu::*;
use std::sync::mpsc::Sender;

/// Internal state of the tray icon.
///
/// This struct holds all the configuration and state for a tray icon,
/// including its appearance, menu items, and event communication channel.
pub struct TrayState {
    /// The name of the icon from the freedesktop icon theme.
    pub icon_name: String,
    /// Path to search for custom icon themes.
    pub icon_theme_path: String,
    /// Raw icon data as pixmaps.
    pub icon_pixmap: Vec<ksni::Icon>,
    /// The title text of the tray icon.
    pub title: String,
    /// Title for the tooltip.
    pub tooltip_title: String,
    /// Subtitle for the tooltip.
    pub tooltip_subtitle: String,
    /// Icon name for the tooltip.
    pub tooltip_icon_name: String,
    /// Unique identifier for this tray icon.
    pub tray_id: String,
    /// Menu structure containing all menu items.
    pub menu: Vec<MenuItemData>,
    /// Channel sender for emitting events to Godot.
    pub event_sender: Option<Sender<TrayEvent>>,
}

impl TrayState {
    /// Creates a new `TrayState` with default values.
    ///
    /// # Parameters
    ///
    /// - `tray_id` - Unique identifier for the tray icon
    pub fn new(tray_id: String) -> Self {
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

    /// Finds a checkmark item by ID and toggles its state.
    ///
    /// Returns the new checked state if found, or None if not found.
    pub fn find_and_toggle_checkmark(&mut self, id: &str) -> Option<bool> {
        Self::find_and_toggle_checkmark_recursive(&mut self.menu, id)
    }

    /// Recursively searches through menu items to find and toggle a checkmark.
    pub fn find_and_toggle_checkmark_recursive(
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

    /// Finds a radio group by ID and selects the option at the given index.
    ///
    /// Returns the ID of the selected option if found, or None if not found.
    pub fn find_and_select_radio(&mut self, group_id: &str, index: usize) -> Option<String> {
        Self::find_and_select_radio_recursive(&mut self.menu, group_id, index)
    }

    /// Recursively searches through menu items to find and select a radio option.
    pub fn find_and_select_radio_recursive(
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

    /// Builds the ksni menu structure from the internal menu data.
    pub fn build_menu_items(&self) -> Vec<MenuItem<KsniTray>> {
        self.menu
            .iter()
            .map(|item| self.build_menu_item(item))
            .collect()
    }

    /// Converts a single MenuItemData into a ksni MenuItem.
    pub fn build_menu_item(&self, item: &MenuItemData) -> MenuItem<KsniTray> {
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
