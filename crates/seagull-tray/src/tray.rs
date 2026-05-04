use std::sync::Arc;

use ksni::menu::{MenuItem, StandardItem};
use ksni::{Category, Icon, Status, Tray};
use tokio::sync::mpsc;

use crate::actions;
use crate::config::Config;
use crate::platform::Opener;

/// Commands emitted by menu clicks. Processed by the controller task in
/// `main.rs`, which owns the serial session and tray handle.
#[derive(Debug)]
pub enum TrayCommand {
    Connect,
    Disconnect,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connected,
}

pub struct SeagullTray {
    state: ConnectionState,
    device_path: Option<String>,
    config: Config,
    opener: Arc<dyn Opener>,
    cmd_tx: mpsc::UnboundedSender<TrayCommand>,
}

impl SeagullTray {
    pub fn new(
        config: Config,
        opener: Arc<dyn Opener>,
        cmd_tx: mpsc::UnboundedSender<TrayCommand>,
    ) -> Self {
        Self {
            state: ConnectionState::Disconnected,
            device_path: None,
            config,
            opener,
            cmd_tx,
        }
    }

    pub fn set_connected(&mut self, device_path: String) {
        self.state = ConnectionState::Connected;
        self.device_path = Some(device_path);
    }

    pub fn set_disconnected(&mut self) {
        self.state = ConnectionState::Disconnected;
        self.device_path = None;
    }
}

impl Tray for SeagullTray {
    fn category(&self) -> Category {
        Category::Hardware
    }

    fn id(&self) -> String {
        "at.vircodific.seagull.Tray".into()
    }

    fn title(&self) -> String {
        match self.state {
            ConnectionState::Connected => format!(
                "Seagull (connected: {})",
                self.device_path.as_deref().unwrap_or("?")
            ),
            ConnectionState::Disconnected => "Seagull (disconnected)".into(),
        }
    }

    fn status(&self) -> Status {
        Status::Active
    }

    fn icon_name(&self) -> String {
        match self.state {
            ConnectionState::Connected => "network-wired-symbolic".into(),
            ConnectionState::Disconnected => "network-offline-symbolic".into(),
        }
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        Vec::new()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: self.icon_name(),
            title: self.title(),
            description: String::new(),
            icon_pixmap: Vec::new(),
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let connect_label = match self.state {
            ConnectionState::Connected => "Disconnect",
            ConnectionState::Disconnected => "Connect",
        };

        let device_path = if let Some(device_path) = &self.device_path {
            device_path
        } else {
            "(none)"
        };

        vec![
            StandardItem {
                label: "Seagull IME".into(),
                enabled: false,
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: device_path.into(),
                enabled: false,
                ..Default::default()
            }.into(),
            StandardItem {
                label: connect_label.into(),
                activate: Box::new(|tray: &mut SeagullTray| {
                    match tray.state {
                        ConnectionState::Connected => {
                            // Update state immediately so ksni picks it up in
                            // its post-click update() call. The controller will
                            // do the actual serial teardown asynchronously.
                            tray.set_disconnected();
                            let _ = tray.cmd_tx.send(TrayCommand::Disconnect);
                        }
                        ConnectionState::Disconnected => {
                            // Don't optimistically flip to Connected here —
                            // we don't know yet whether the serial open will
                            // succeed. The controller calls set_connected() on
                            // success.
                            let _ = tray.cmd_tx.send(TrayCommand::Connect);
                        }
                    }
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Open Dictionary".into(),
                activate: Box::new(|tray: &mut SeagullTray| {
                    let opener = tray.opener.clone();
                    let config = tray.config.clone();
                    actions::spawn_open(opener, move |o| actions::open_dictionary(o, &config));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Open Settings".into(),
                activate: Box::new(|tray: &mut SeagullTray| {
                    let opener = tray.opener.clone();
                    let config = tray.config.clone();
                    actions::spawn_open(opener, move |o| actions::open_settings(o, &config));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Open Log".into(),
                activate: Box::new(|tray: &mut SeagullTray| {
                    let opener = tray.opener.clone();
                    actions::spawn_open(opener, |o| actions::open_log(o));
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|tray: &mut SeagullTray| {
                    let _ = tray.cmd_tx.send(TrayCommand::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
