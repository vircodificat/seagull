mod buffer;
mod config;
mod control;
mod engine;
mod notifications;

use std::sync::Arc;

use log::{error, info, warn};
use simplelog::{Config as LogConfig, LevelFilter, WriteLogger};

use seagull::{JsonDictionary, Key, Stroke};
use tokio::sync::Mutex;
use zbus::connection::Builder;
use zbus::zvariant::ObjectPath;

use buffer::{StrokeBuffer, SearchState};
use config::Config;
use control::Control;
use engine::{emit_auxiliary_text, emit_for_action, hide_auxiliary_text, Engine, Factory, SharedConnection, SharedHintState, SharedSearchState};

const ENGINE_PATH: &str = "/org/freedesktop/IBus/Engine/SeagullIME";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";
const CONTROL_PATH: &str = "/at/vircodific/seagull/IME";
const CONTROL_BUS_NAME: &str = "at.vircodific.seagull.IME";

fn init_log() {
    let log_dir = std::env::var("HOME")
        .map(|h| format!("{h}/.local/share/seagull-ime"))
        .unwrap_or_else(|_| "/tmp".to_string());
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = format!("{log_dir}/seagull-ime.log");
    if let Ok(f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        WriteLogger::init(LevelFilter::Info, LogConfig::default(), f).ok();
    }
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
    init_log();

    std::panic::set_hook(Box::new(|info| {
        eprintln!("PANIC: {info}");
    }));

    info!("SeagullIME starting");

    let config = Config::load()?;

    info!("Config: dict={}", config.dictionary.path);

    info!("Loading dictionary from {}", config.dictionary.path);
    let dict_path_for_error = config.dictionary.path.clone();
    let dictionary = match JsonDictionary::load_from_file(&config.dictionary.path) {
        Ok(d) => {
            info!("Dictionary loaded successfully");
            d
        }
        Err(e) => {
            error!("FATAL: Failed to load dictionary: {e}");
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

    // Strokes arrive over D-Bus from `seagull-tray` via the Control interface;
    // they're forwarded onto this channel and consumed by the main select! loop.
    let (stroke_tx, mut stroke_rx) = tokio::sync::mpsc::channel::<(Stroke, bool)>(64);

    let engine_obj_path: ObjectPath<'static> = ENGINE_PATH.try_into()?;
    let factory = Factory::new(engine_obj_path.clone());

    // Create a shared connection reference that will be set after the connection is built
    let shared_connection: SharedConnection = Arc::new(Mutex::new(None));
    let hint_showing: SharedHintState = Arc::new(Mutex::new(false));
    let search_state: SharedSearchState = Arc::new(Mutex::new(SearchState::Inactive));
    let engine = Engine::new(buffer.clone(), shared_connection.clone(), hint_showing.clone(), search_state.clone());

    let ibus_addr = std::env::var("IBUS_ADDRESS")
        .ok()
        .or_else(|| discover_ibus_address());
    info!("IBus address: {:?}", ibus_addr);

    let builder = if let Some(ref addr) = ibus_addr {
        info!("Connecting to IBus bus at {addr}");
        Builder::address(addr.as_str())?
    } else {
        warn!("Could not find IBus address, falling back to session bus");
        Builder::session()?
    };

    let connection = match builder
        .name("at.vircodific.seagull.IBus")?
        .serve_at(FACTORY_PATH, factory)?
        .serve_at(ENGINE_PATH, engine.clone())?
        .build()
        .await
    {
        Ok(c) => {
            info!("D-Bus connection established");
            c
        }
        Err(e) => {
            error!("FATAL: D-Bus connection failed: {e}");
            return Err(e.into());
        }
    };

    // Store the connection in the shared reference
    {
        let mut conn_ref = shared_connection.lock().await;
        *conn_ref = Some(connection.clone());
    }

    // Build a separate session bus connection that owns the Control interface
    // and claims `at.vircodific.seagull.IME`. The IBus connection above is on its own
    // bus and isn't reachable from `seagull-tray`. The session connection is
    // also reused for desktop notifications.
    let _control_connection = match Builder::session()?
        .name(CONTROL_BUS_NAME)?
        .serve_at(CONTROL_PATH, Control::new(stroke_tx.clone()))?
        .build()
        .await
    {
        Ok(c) => {
            info!("Control D-Bus interface registered at {CONTROL_PATH} on {CONTROL_BUS_NAME}");
            c
        }
        Err(e) => {
            error!("FATAL: failed to register Control D-Bus interface: {e}");
            return Err(e.into());
        }
    };

    // Drop the spare sender so the channel closes if the Control object goes
    // away (e.g. the connection drops); the main loop will then exit cleanly.
    drop(stroke_tx);

    info!("Ready, waiting for strokes...");
    loop {
        tokio::select! {
            Some((stroke, is_control)) = stroke_rx.recv() => {
                info!("Processing stroke: {stroke} (control={is_control})");

                // Control + H: show "HINT" auxiliary text popup.
                if is_control && stroke == Stroke::new(&[Key::LeftH]) {
                    info!("  Control+H: showing HINT");
                    if let Err(e) = emit_auxiliary_text(&connection, "HINT").await {
                        error!("  ERROR showing hint: {e}");
                    } else {
                        *hint_showing.lock().await = true;
                    }
                    continue;
                }

                // Any other stroke dismisses the hint popup. Emit the hide
                // signal unconditionally (when not in search mode) so we are
                // robust to `hint_showing` being out of sync with the actual
                // popup state — a keyboard event arriving between the show
                // signal and the flag assignment above, or a keyboard event
                // calling `engine.hide_hint()` in a separate task, can leave
                // the flag false even though the popup is still on screen.
                {
                    let in_search = matches!(
                        *search_state.lock().await,
                        SearchState::Active(_)
                    );
                    if !in_search {
                        let mut showing = hint_showing.lock().await;
                        let was_showing = *showing;
                        *showing = false;
                        drop(showing);
                        if was_showing {
                            info!("  Dismissing hint due to stroke");
                        }
                        if let Err(e) = hide_auxiliary_text(&connection).await {
                            warn!("  Failed to hide hint: {e}");
                        }
                    }
                }

                // Control + S: activate search mode
                if is_control && stroke == Stroke::new(&[Key::LeftS]) {
                    info!("  Control+S: activating search mode");
                    if let Err(e) = engine.show_search().await {
                        error!("  ERROR activating search: {e}");
                    }
                    continue;
                }

                if is_control && stroke == Stroke::star() {
                    let mut buf = buffer.lock().await;
                    buf.clear();
                    info!("  Buffer cleared (control+star)");
                    if let Err(e) = emit_for_action(
                        &buffer::BufferAction::UpdatePreedit, "", &connection,
                    ).await {
                        error!("  ERROR emitting signal: {e}");
                    }
                    continue;
                }

                if is_control {
                    info!("  Skipping control stroke");
                    continue;
                }

                // Skip normal stroke processing if search is active (keyboard input will be handled separately)
                {
                    let search = search_state.lock().await;
                    if matches!(*search, SearchState::Active(_)) {
                        info!("  Skipping stroke while search is active");
                        continue;
                    }
                }

                let action = {
                    let mut buf = buffer.lock().await;
                    buf.push_stroke(stroke)
                };

                let preedit = {
                    let buf = buffer.lock().await;
                    buf.preedit_string()
                };

                info!("  Action: {action:?}, preedit: \"{preedit}\"");
                info!("  Calling emit_for_action...");

                if let Err(e) = emit_for_action(&action, &preedit, &connection).await {
                    error!("  ERROR emitting signal: {e}");
                } else {
                    info!("  Signals emitted successfully");
                }
            }
            else => {
                info!("Stroke channel closed, exiting");
                break;
            }
        }
    }

    info!("Stroke channel closed, exiting");
    Ok(())
}
