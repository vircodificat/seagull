mod buffer;
mod config;
mod engine;
mod notifications;

use std::io::Write;
use std::sync::Arc;

use seagull::device::serial::SerialDevice;
use seagull::device::{Device, Keycode};
use seagull::{JsonDictionary, Stroke};
use tokio::sync::Mutex;
use zbus::connection::Builder;
use zbus::zvariant::ObjectPath;

use buffer::StrokeBuffer;
use config::Config;
use engine::{emit_for_action, Engine, Factory, SharedConnection};

const ENGINE_PATH: &str = "/org/freedesktop/IBus/Engine/SeagullIME";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

fn setup_log() -> Box<dyn Write + Send> {
    let log_dir = std::env::var("HOME")
        .map(|h| format!("{h}/.local/share/seagull-ime"))
        .unwrap_or_else(|_| "/tmp".to_string());
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = format!("{log_dir}/seagull-ime.log");
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(f) => Box::new(f),
        Err(_) => Box::new(std::io::stderr()),
    }
}

macro_rules! log {
    ($logger:expr, $($arg:tt)*) => {{
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!($logger, "[{now}] {}", format!($($arg)*));
        let _ = $logger.flush();
    }};
}

fn discover_ibus_address() -> Option<String> {
    if let Ok(home) = std::env::var("HOME") {
        let bus_dir = format!("{home}/.config/ibus/bus");
        if let Ok(entries) = std::fs::read_dir(&bus_dir) {
            for entry in entries.flatten() {
                if let Ok(contents) = std::fs::read_to_string(entry.path()) {
                    for line in contents.lines() {
                        if let Some(addr) = line.strip_prefix("IBUS_ADDRESS=") {
                            return Some(addr.to_string());
                        }
                    }
                }
            }
        }
    }

    std::process::Command::new("ibus")
        .arg("address")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::panic::set_hook(Box::new(|info| {
        let mut l = setup_log();
        let _ = writeln!(l, "PANIC: {info}");
        let _ = l.flush();
    }));

    let mut logger = setup_log();
    log!(logger, "SeagullIME starting");

    let config = Config::load()?;

    let candidates = config.device_candidates();

    let mut serial_device_path = match Config::try_connect_device(&candidates[0]) {
        Some(path) => path,
        None => {
            log!(logger, "FATAL: Failed to connect to any serial device");
            return Err("Failed to connect to any serial device".into());
        }
    };

    log!(logger, "Config: dict={}, serial={}", config.dictionary.path, serial_device_path);

    log!(logger, "Loading dictionary from {}", config.dictionary.path);
    let dict_path_for_error = config.dictionary.path.clone();
    let dictionary = match JsonDictionary::load_from_file(&config.dictionary.path) {
        Ok(d) => {
            log!(logger, "Dictionary loaded successfully");
            d
        }
        Err(e) => {
            log!(logger, "FATAL: Failed to load dictionary: {e}");
            eprintln!("ERROR: Dictionary file not found at: {}", config.dictionary.path);

            // Try to send a notification via D-Bus before exiting
            let path_for_notif = dict_path_for_error.clone();
            tokio::spawn(async move {
                if let Ok(conn) = zbus::Connection::session().await {
                    if let Err(e) = notifications::dictionary_not_found(&conn, &path_for_notif).await {
                        eprintln!("Failed to send dictionary not found notification: {e}");
                    }
                }
            });

            // Give the notification a moment to send
            std::thread::sleep(std::time::Duration::from_millis(500));

            return Err(format!("Dictionary file not found: {}", config.dictionary.path).into());
        }
    };

    let buffer = Arc::new(Mutex::new(StrokeBuffer::new(dictionary)));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Keycode>(64);

    // Channel for notifications from the serial thread
    #[derive(Clone)]
    enum NotificationEvent {
        DeviceDisconnected,
        DeviceReconnected,
    }
    let (notif_tx, mut notif_rx) = tokio::sync::mpsc::channel::<NotificationEvent>(16);

    let engine_obj_path: ObjectPath<'static> = ENGINE_PATH.try_into()?;
    let factory = Factory::new(engine_obj_path.clone());

    // Create a shared connection reference that will be set after the connection is built
    let shared_connection: SharedConnection = Arc::new(Mutex::new(None));
    let engine = Engine::new(buffer.clone(), shared_connection.clone());

    let ibus_addr = std::env::var("IBUS_ADDRESS")
        .ok()
        .or_else(|| discover_ibus_address());
    log!(logger, "IBus address: {:?}", ibus_addr);

    let builder = if let Some(ref addr) = ibus_addr {
        log!(logger, "Connecting to IBus bus at {addr}");
        Builder::address(addr.as_str())?
    } else {
        log!(logger, "WARNING: Could not find IBus address, falling back to session bus");
        Builder::session()?
    };

    let connection = match builder
        .name("org.freedesktop.IBus.SeagullIME")?
        .serve_at(FACTORY_PATH, factory)?
        .serve_at(ENGINE_PATH, engine)?
        .build()
        .await
    {
        Ok(c) => {
            log!(logger, "D-Bus connection established");
            c
        }
        Err(e) => {
            log!(logger, "FATAL: D-Bus connection failed: {e}");
            return Err(e.into());
        }
    };

    // Store the connection in the shared reference
    {
        let mut conn_ref = shared_connection.lock().await;
        *conn_ref = Some(connection.clone());
    }

    // Spawn serial device reader thread
    let candidates_clone = candidates.clone();
    let mut serial_logger = setup_log();
    let notif_tx_clone = notif_tx.clone();
    std::thread::spawn(move || {
        log!(serial_logger, "Opening serial device {serial_device_path}");
        let mut device = match SerialDevice::new(&serial_device_path) {
            Ok(d) => {
                log!(serial_logger, "Serial device opened successfully");
                d
            }
            Err(e) => {
                log!(serial_logger, "FATAL: Failed to open serial device: {e}");
                return;
            }
        };

        loop {
            match device.read_stroke() {
                Ok(keycode) => {
                    let stroke = keycode.stroke();
                    log!(serial_logger, "Stroke received: {stroke} (control={})", keycode.is_control());
                    if tx.blocking_send(keycode).is_err() {
                        log!(serial_logger, "Channel closed, serial reader exiting");
                        break;
                    }
                }
                Err(e) => {
                    log!(serial_logger, "Serial read error: {e}, device disconnected");
                    let _ = notif_tx_clone.blocking_send(NotificationEvent::DeviceDisconnected);

                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(1));

                        let mut reconnected = false;
                        for candidate in &candidates_clone {
                            if let Some(path) = Config::try_connect_device(candidate) {
                                log!(serial_logger, "Device reconnected as {}", path);
                                serial_device_path = path;
                                reconnected = true;
                                break;
                            }
                        }

                        if reconnected {
                            device = match SerialDevice::new(&serial_device_path) {
                                Ok(d) => {
                                    let _ = notif_tx_clone.blocking_send(NotificationEvent::DeviceReconnected);
                                    d
                                },
                                Err(e) => {
                                    log!(serial_logger, "Failed to reopen device: {e}");
                                    continue;
                                }
                            };
                            break;
                        } else {
                            log!(serial_logger, "Still disconnected, retrying...");
                        }
                    }
                }
            }
        }
    });

    // Create a separate session bus connection for notifications
    let notif_connection = match zbus::Connection::session().await {
        Ok(c) => {
            log!(logger, "Session bus connection for notifications established");
            eprintln!("✓ Session bus connection for notifications established");
            c
        }
        Err(e) => {
            log!(logger, "WARNING: Failed to create session bus connection for notifications: {e}");
            eprintln!("✗ Failed to create session bus for notifications: {e}");
            // Create a dummy connection - we'll skip notifications if this fails
            connection.clone()
        }
    };

    log!(logger, "Ready, waiting for strokes...");
    loop {
        tokio::select! {
            Some(keycode) = rx.recv() => {
                let stroke = keycode.stroke();
                log!(logger, "Processing stroke: {stroke} (control={})", keycode.is_control());

                if keycode.is_control() && stroke == Stroke::star() {
                    let mut buf = buffer.lock().await;
                    buf.clear();
                    log!(logger, "  Buffer cleared (control+star)");
                    if let Err(e) = emit_for_action(
                        &buffer::BufferAction::UpdatePreedit, "", &connection,
                    ).await {
                        log!(logger, "  ERROR emitting signal: {e}");
                    }
                    continue;
                }

                if keycode.is_control() {
                    log!(logger, "  Skipping control stroke");
                    continue;
                }

                let action = {
                    let mut buf = buffer.lock().await;
                    buf.push_stroke(stroke)
                };

                let preedit = {
                    let buf = buffer.lock().await;
                    buf.preedit_string()
                };

                log!(logger, "  Action: {action:?}, preedit: \"{preedit}\"");
                log!(logger, "  Calling emit_for_action...");

                if let Err(e) = emit_for_action(&action, &preedit, &connection).await {
                    log!(logger, "  ERROR emitting signal: {e}");
                } else {
                    log!(logger, "  Signals emitted successfully");
                }
            }
            Some(notif_event) = notif_rx.recv() => {
                match notif_event {
                    NotificationEvent::DeviceDisconnected => {
                        log!(logger, "Received disconnect notification event, calling device_disconnected()");
                        eprintln!("→ Sending device disconnected notification...");
                        match notifications::device_disconnected(&notif_connection).await {
                            Ok(_) => {
                                log!(logger, "Device disconnected notification sent successfully");
                                eprintln!("✓ Disconnect notification sent");
                            }
                            Err(e) => {
                                log!(logger, "Failed to send disconnect notification: {e}");
                                eprintln!("✗ Failed to send disconnect notification: {e}");
                            }
                        }
                    }
                    NotificationEvent::DeviceReconnected => {
                        log!(logger, "Received reconnect notification event, calling device_reconnected()");
                        eprintln!("→ Sending device reconnected notification...");
                        match notifications::device_reconnected(&notif_connection).await {
                            Ok(_) => {
                                log!(logger, "Device reconnected notification sent successfully");
                                eprintln!("✓ Reconnect notification sent");
                            }
                            Err(e) => {
                                log!(logger, "Failed to send reconnect notification: {e}");
                                eprintln!("✗ Failed to send reconnect notification: {e}");
                            }
                        }
                    }
                }
            }
            else => {
                log!(logger, "All channels closed, exiting");
                break;
            }
        }
    }

    log!(logger, "Stroke channel closed, exiting");
    Ok(())
}
