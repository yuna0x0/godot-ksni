use godot::prelude::*;
use image::GenericImageView;
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

    #[func]
    fn update_tray(&mut self) {
        if let Some(handle) = &self.handle {
            handle.update(|_tray: &mut KsniTray| {});
        }
    }

    #[func]
    fn set_tray_id(&mut self, tray_id: GString) {
        let mut state = self.state.lock().unwrap();
        state.tray_id = tray_id.to_string();
    }

    #[func]
    fn set_icon_name(&mut self, icon_name: GString) {
        let mut state = self.state.lock().unwrap();
        state.icon_name = icon_name.to_string();
    }

    #[func]
    fn set_icon_theme_path(&mut self, path: GString) {
        let mut state = self.state.lock().unwrap();
        state.icon_theme_path = path.to_string();
    }

    #[func]
    fn set_icon_from_file(&mut self, path: GString) -> bool {
        let path_str = path.to_string();

        let bytes = match std::fs::read(&path_str) {
            Ok(b) => b,
            Err(e) => {
                godot_error!("Failed to read file {}: {}", path_str, e);
                return false;
            }
        };

        let img = match image::load_from_memory(&bytes) {
            Ok(img) => img,
            Err(e) => {
                godot_error!("Failed to load icon from file {}: {}", path_str, e);
                return false;
            }
        };

        let (width, height) = img.dimensions();
        let mut data = img.into_rgba8().into_vec();
        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }

        let mut state = self.state.lock().unwrap();
        state.icon_pixmap = vec![ksni::Icon {
            width: width as i32,
            height: height as i32,
            data,
        }];
        state.icon_name = String::new();
        true
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

    #[func]
    fn set_title(&mut self, title: GString) {
        let mut state = self.state.lock().unwrap();
        state.title = title.to_string();
    }

    #[func]
    fn set_tooltip(&mut self, title: GString, subtitle: GString, icon_name: GString) {
        let mut state = self.state.lock().unwrap();
        state.tooltip_title = title.to_string();
        state.tooltip_subtitle = subtitle.to_string();
        state.tooltip_icon_name = icon_name.to_string();
    }

    #[func]
    fn clear_menu(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.menu.clear();
    }

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

    #[func]
    fn add_radio_group(&mut self, id: GString, selected: i64) {
        let mut state = self.state.lock().unwrap();
        state.menu.push(MenuItemData::RadioGroup {
            id: id.to_string(),
            selected: selected as usize,
            options: Vec::new(),
        });
    }

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

    #[func]
    fn add_separator(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.menu.push(MenuItemData::Separator);
    }

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

struct GodotKsniExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotKsniExtension {}
