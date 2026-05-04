use std::sync::Arc;

use ksni::menu::{MenuItem, StandardItem, SubMenu};
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connected,
}

pub struct SeagullTray {
    state: ConnectionState,
    device_path: String,
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
        let device_path = config.device_candidates().first().cloned().unwrap_or_else(|| "/dev/null".to_string());
        Self {
            state: ConnectionState::Disconnected,
            device_path,
            config,
            opener,
            cmd_tx,
        }
    }

    pub fn set_connected(&mut self, device_path: String) {
        self.state = ConnectionState::Connected;
        self.device_path = device_path;
    }

    pub fn set_disconnected(&mut self) {
        self.state = ConnectionState::Disconnected;
        self.device_path = "/dev/null".to_string();
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
                self.device_path
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

        let device_path = self.device_path.as_str();

        let mut devices = vec![];
        for device in &self.config.device.devices {
            devices.push(device.clone());
        }

        let device_submenu =
            SubMenu {
                label: device_path.into(),
                submenu: devices.into_iter().map(|device_path| {
                    StandardItem {
                        label: device_path.clone().into(),
                        enabled: true,
                        activate: Box::new(move |tray: &mut SeagullTray| {
                            tray.device_path = device_path.clone();
                        }),
                        ..Default::default()
                    }.into()
                }).collect::<Vec<_>>(),
                ..Default::default()
            };

        vec![
            StandardItem {
                label: "Seagull IME".into(),
                enabled: false,
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            device_submenu.into(),
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
                label: "Open Log Folder".into(),
                activate: Box::new(|tray: &mut SeagullTray| {
                    let opener = tray.opener.clone();
                    actions::spawn_open(opener, |o| actions::open_log(o));
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
