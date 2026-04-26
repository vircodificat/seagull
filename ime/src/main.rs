mod buffer;
mod engine;

use std::io::Write;
use std::sync::Arc;

use seagull::device::serial::SerialDevice;
use seagull::device::{Device, Keycode};
use seagull::{JsonDictionary, Stroke};
use tokio::sync::Mutex;
use zbus::connection::Builder;
use zbus::zvariant::ObjectPath;

use buffer::StrokeBuffer;
use engine::{emit_for_action, Engine, Factory};

const DEFAULT_SERIAL_DEVICE: &str =
    "/dev/serial/by-id/usb-Wootpatoot_Lets_Split_v2-if02";
const ENGINE_PATH: &str = "/org/freedesktop/IBus/Engine/SeagullIME";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

/// Simple file logger. Writes to ~/.local/share/seagull-ime/seagull-ime.log.
/// Falls back to stderr if the log file can't be opened.
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

/// Discover the IBus D-Bus address from the filesystem or `ibus address` command.
fn discover_ibus_address() -> Option<String> {
    // Try reading from ~/.config/ibus/bus/ directory.
    // IBus writes a file named like {machine-id}-{display} in there.
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

    // Fall back to running `ibus address`.
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
    // Install panic hook that logs to our log file
    std::panic::set_hook(Box::new(|info| {
        let mut l = setup_log();
        let _ = writeln!(l, "PANIC: {info}");
        let _ = l.flush();
    }));

    let mut logger = setup_log();
    log!(logger, "SeagullIME starting");

    // --- Configuration from environment ---
    let dict_path = std::env::var("SEAGULL_DICTIONARY_PATH").unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME not set");
        format!("{home}/.config/seagull/seagull.json")
    });

    let serial_device = std::env::var("SEAGULL_SERIAL_DEVICE")
        .unwrap_or_else(|_| DEFAULT_SERIAL_DEVICE.to_string());

    log!(logger, "Config: dict={dict_path}, serial={serial_device}");

    // --- Load dictionary ---
    log!(logger, "Loading dictionary from {dict_path}");
    let dictionary = match JsonDictionary::load_from_file(&dict_path) {
        Ok(d) => {
            log!(logger, "Dictionary loaded successfully");
            d
        }
        Err(e) => {
            log!(logger, "FATAL: Failed to load dictionary: {e}");
            return Err(e);
        }
    };

    // --- Create buffer ---
    let buffer = Arc::new(Mutex::new(StrokeBuffer::new(dictionary)));

    // --- Stroke channel (serial thread → async task) ---
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Keycode>(64);

    // --- Serial reader thread (blocking) ---
    let serial_device_path = serial_device.clone();
    let mut serial_logger = setup_log();
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
            let keycode = device.read_stroke();
            let stroke = keycode.stroke();
            log!(serial_logger, "Stroke received: {stroke} (control={})", keycode.is_control());
            if tx.blocking_send(keycode).is_err() {
                log!(serial_logger, "Channel closed, serial reader exiting");
                break;
            }
        }
    });

    // --- D-Bus connection ---
    let engine_obj_path: ObjectPath<'static> = ENGINE_PATH.try_into()?;
    let factory = Factory::new(engine_obj_path.clone());
    let engine = Engine::new(buffer.clone());

    // Connect to the IBus bus. Try IBUS_ADDRESS env var first, then discover
    // from the address file, then fall back to `ibus address` command.
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

    // --- Stroke processing loop ---
    log!(logger, "Ready, waiting for strokes...");
    while let Some(keycode) = rx.recv().await {
        let stroke = keycode.stroke();
        log!(logger, "Processing stroke: {stroke} (control={})", keycode.is_control());

        // Control + star: clear the buffer.
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

        // Skip all other control strokes.
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

    log!(logger, "Stroke channel closed, exiting");
    Ok(())
}
