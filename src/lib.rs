use godot::prelude::*;
use ksni::blocking::TrayMethods;
use ksni::menu::*;
use std::sync::{Arc, Mutex};

struct TrayState {
    icon_name: String,
    title: String,
    menu_items: Vec<String>,
    tray_id: String,
}

impl TrayState {
    fn new(tray_id: String) -> Self {
        Self {
            icon_name: "application-x-executable".to_string(),
            title: "Tray Icon".to_string(),
            menu_items: Vec::new(),
            tray_id,
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

    fn title(&self) -> String {
        let state = self.state.lock().unwrap();
        state.title.clone()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let state = self.state.lock().unwrap();
        let mut items = Vec::new();

        for label in &state.menu_items {
            items.push(
                StandardItem {
                    label: label.clone(),
                    activate: Box::new(|_this: &mut Self| {}),
                    ..Default::default()
                }
                .into(),
            );
        }

        if items.is_empty() {
            items.push(
                StandardItem {
                    label: "No items".into(),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        }

        items
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct TrayIcon {
    base: Base<Node>,
    handle: Option<ksni::blocking::Handle<KsniTray>>,
    state: Arc<Mutex<TrayState>>,
}

#[godot_api]
impl INode for TrayIcon {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            handle: None,
            state: Arc::new(Mutex::new(TrayState::new("godot_tray_icon".to_string()))),
        }
    }

    fn ready(&mut self) {
        godot_print!("TrayIcon ready");
    }
}

#[godot_api]
impl TrayIcon {
    #[func]
    fn spawn_tray(&mut self) {
        if self.handle.is_some() {
            godot_print!("Tray already spawned");
            return;
        }

        let state = self.state.clone();
        let tray = KsniTray { state };

        std::thread::spawn(move || match tray.spawn() {
            Ok(_handle) => {
                godot_print!("Tray spawned successfully");
                loop {
                    std::thread::park();
                }
            }
            Err(e) => {
                godot_error!("Failed to spawn tray: {}", e);
            }
        });

        godot_print!("Tray spawn initiated");
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
    fn set_title(&mut self, title: GString) {
        let mut state = self.state.lock().unwrap();
        state.title = title.to_string();
    }

    #[func]
    fn add_menu_item(&mut self, label: GString) {
        let mut state = self.state.lock().unwrap();
        state.menu_items.push(label.to_string());
    }

    #[func]
    fn clear_menu(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.menu_items.clear();
    }
}
