//! KSNI tray bridge implementation.
//!
//! This module provides the bridge between our internal tray state and the ksni library,
//! implementing the `ksni::Tray` trait to connect with the StatusNotifierItem specification.

use crate::tray::state::TrayState;
use ksni::menu::MenuItem;
use std::sync::{Arc, Mutex};

/// Implementation of the ksni::Tray trait that bridges our internal state
/// with the ksni library.
///
/// This struct wraps the shared tray state and implements all the required
/// methods for the StatusNotifierItem specification.
pub struct KsniTray {
    /// Shared reference to the tray state.
    pub state: Arc<Mutex<TrayState>>,
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
