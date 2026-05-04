mod actions;
mod config;
mod ime_client;
mod platform;
mod serial;
mod tray;

use std::sync::Arc;

use ksni::TrayMethods;
use log::{error, info, warn};
use simplelog::{Config as LogConfig, LevelFilter, WriteLogger};
use tokio::sync::mpsc;

use crate::config::Config;
use crate::ime_client::{ImeClient, spawn_name_watcher};
use crate::platform::DefaultOpener;
use crate::serial::Session;
use crate::tray::{SeagullTray, TrayCommand};

const STROKE_CHANNEL_CAPACITY: usize = 64;

fn init_log() {
    let log_path = Config::log_dir_path();
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        WriteLogger::init(LevelFilter::Info, LogConfig::default(), f).ok();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_log();
    info!("seagull-tray starting");

    let config = Config::load();
    let opener: Arc<dyn platform::Opener> = Arc::new(DefaultOpener);

    let connection = zbus::Connection::session().await?;
    let ime_client = Arc::new(ImeClient::new(&connection).await?);
    spawn_name_watcher(connection.clone(), ime_client.clone());

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<TrayCommand>();
    let tray = SeagullTray::new(config.clone(), opener.clone(), cmd_tx.clone());
    let tray_handle = tray.spawn().await?;

    let mut session: Option<Session> = None;

    loop {
        let cmd = match cmd_rx.recv().await {
            Some(c) => c,
            None => {
                warn!("Tray command channel closed; exiting");
                break;
            }
        };

        match cmd {
            TrayCommand::Connect => {
                if session.is_some() {
                    info!("Already connected; ignoring Connect");
                    continue;
                }
                let (stroke_tx, stroke_rx) =
                    mpsc::channel(STROKE_CHANNEL_CAPACITY);
                match serial::open_session(&config, stroke_tx, cmd_tx.clone()) {
                    Ok(new_session) => {
                        let path = new_session.device_path().to_string();
                        info!("Connected: {path}");
                        session = Some(new_session);
                        let client = ime_client.clone();
                        tokio::spawn(serial::forward_strokes(stroke_rx, client));
                        let path_for_tray = path.clone();
                        tray_handle
                            .update(move |t: &mut SeagullTray| {
                                t.set_connected(path_for_tray);
                            })
                            .await;
                    }
                    Err(e) => {
                        error!("Connect failed: {e}");
                    }
                }
            }
            TrayCommand::Disconnect => {
                if let Some(s) = session.take() {
                    info!("Disconnecting...");
                    s.stop();
                    tray_handle
                        .update(|t: &mut SeagullTray| {
                            info!("Setting tray to disconnected");
                            t.set_disconnected()
                        })
                        .await;
                    info!("Disconnect complete");
                } else {
                    info!("Already disconnected; ignoring Disconnect");
                }
            }
        }
    }

    info!("seagull-tray exiting");
    Ok(())
}
