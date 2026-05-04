use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use log::{error, info, warn};
use seagull::device::serial::SerialDevice;
use seagull::device::Device;
use seagull_ipc::ImeMessage;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::ime_client::ImeClient;
use crate::tray::TrayCommand;

/// Owns the live serial reader thread. Dropping the session signals the
/// reader to stop on its next loop iteration; no auto-reconnect.
pub struct Session {
    device_path: String,
    should_stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Session {
    pub fn device_path(&self) -> &str {
        &self.device_path
    }

    pub fn stop(mut self) {
        self.should_stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            // Reader uses a 10ms read timeout, so this returns quickly.
            let _ = handle.join();
        }
        info!("Serial session stopped: {}", self.device_path);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.join.is_some() {
            self.should_stop.store(true, Ordering::Relaxed);
        }
    }
}

/// Open a session whose reader thread streams strokes onto `tx` until stopped
/// or an I/O error occurs. Returns the live `Session` on success.
pub fn open_session(
    config: &Config,
    tx: mpsc::Sender<ImeMessage>,
    cmd_tx: mpsc::UnboundedSender<TrayCommand>,
) -> Result<Session, String> {
    let candidates = config.device_candidates();
    if candidates.is_empty() {
        return Err("no candidate serial devices".into());
    }
    let mut last_err: Option<String> = None;
    for path in candidates {
        info!("Trying serial device: {path}");
        match SerialDevice::new(&path) {
            Ok(device) => {
                let should_stop = Arc::new(AtomicBool::new(false));
                let join = spawn_reader_thread(
                    path.clone(),
                    device,
                    tx.clone(),
                    should_stop.clone(),
                    cmd_tx.clone(),
                );
                return Ok(Session {
                    device_path: path,
                    should_stop,
                    join: Some(join),
                });
            }
            Err(e) => {
                warn!("Failed to open {path}: {e}");
                last_err = Some(format!("{path}: {e}"));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "no candidate serial devices".into()))
}

fn spawn_reader_thread(
    path: String,
    mut device: SerialDevice,
    tx: mpsc::Sender<ImeMessage>,
    should_stop: Arc<AtomicBool>,
    cmd_tx: mpsc::UnboundedSender<TrayCommand>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        info!("Serial reader started: {path}");
        loop {
            if should_stop.load(Ordering::Relaxed) {
                info!("Serial reader stopping (requested): {path}");
                break;
            }
            match device.read_stroke() {
                Ok(keycode) => {
                    let msg = ImeMessage::Stroke {
                        bits: keycode.stroke().bits(),
                        is_control: keycode.is_control(),
                    };
                    if tx.blocking_send(msg).is_err() {
                        info!("Stroke channel closed; serial reader exiting");
                        break;
                    }
                }
                Err(e) => {
                    error!("Serial read error on {path}: {e}");
                    if !should_stop.load(Ordering::Relaxed) {
                        let _ = cmd_tx.send(TrayCommand::Disconnect);
                    }
                    break;
                }
            }
        }
        info!("Serial reader exited: {path}");
    })
}

/// Drains the stroke channel and forwards each message to the IME client.
pub async fn forward_strokes(mut rx: mpsc::Receiver<ImeMessage>, client: Arc<ImeClient>) {
    while let Some(msg) = rx.recv().await {
        client.send(msg).await;
    }
    info!("Stroke forwarder: channel closed");
}


