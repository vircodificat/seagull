use std::io::Write;
use std::sync::Arc;

use tokio::sync::Mutex;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, Value};
use zbus::interface;

use crate::buffer::{BufferAction, StrokeBuffer};

macro_rules! log {
    ($logger:expr, $($arg:tt)*) => {{
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!($logger, "[{now}] ENGINE: {}", format!($($arg)*));
        let _ = $logger.flush();
    }};
}

fn open_log() -> Box<dyn Write + Send> {
    let log_dir = std::env::var("HOME")
        .map(|h| format!("{h}/.local/share/seagull-ime"))
        .unwrap_or_else(|_| "/tmp".to_string());
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

/// Serialize a string as an IBus text variant for D-Bus.
///
/// IBusText GVariant format: `(sa{sv}sv)` where:
///   - s: "IBusText"
///   - a{sv}: attachments (empty dict)
///   - s: the text content
///   - v: variant wrapping IBusAttrList `(sa{sv}av)`
///
/// IBusAttrList GVariant format: `(sa{sv}av)` where:
///   - s: "IBusAttrList"
///   - a{sv}: attachments (empty dict)
///   - av: array of attribute variants (empty)
fn ibus_text(text: &str) -> Value<'static> {
    use std::collections::HashMap;

    // Empty attachments: a{sv} — use a concrete HashMap which serializes as a{sv}
    let empty_attachments: HashMap<String, Value<'static>> = HashMap::new();

    // IBusAttrList: (sa{sv}av) — attributes array is empty Vec<Value>
    let attrs: Vec<Value<'static>> = vec![];
    let attr_list = (
        "IBusAttrList".to_string(),
        empty_attachments.clone(),
        attrs,
    );

    // IBusText: (sa{sv}sv) — the attr_list must be wrapped in a variant
    let ibus_text = (
        "IBusText".to_string(),
        empty_attachments,
        text.to_string(),
        Value::new(attr_list),
    );

    Value::new(ibus_text)
}

/// IBus Engine Factory — creates engine instances on demand.
pub struct Factory {
    engine_path: ObjectPath<'static>,
}

impl Factory {
    pub fn new(engine_path: ObjectPath<'static>) -> Self {
        Self { engine_path }
    }
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl Factory {
    /// Called by IBus to create an engine instance.
    async fn create_engine(&self, name: &str) -> ObjectPath<'static> {
        let mut l = open_log();
        log!(l, "CreateEngine called with name={name}, returning {:?}", self.engine_path);
        self.engine_path.clone()
    }
}

/// IBus Engine — handles input method lifecycle and emits text signals.
pub struct Engine {
    pub buffer: Arc<Mutex<StrokeBuffer>>,
}

impl Engine {
    pub fn new(buffer: Arc<Mutex<StrokeBuffer>>) -> Self {
        Self { buffer }
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl Engine {
    /// IBus calls this for each key event. We always return false (passthrough)
    /// because steno input comes from the serial device, not the keyboard.
    async fn process_key_event(
        &self,
        _keyval: u32,
        _keycode: u32,
        _state: u32,
    ) -> bool {
        false
    }

    async fn focus_in(&self) {
        let mut l = open_log();
        log!(l, "FocusIn");
    }

    async fn focus_out(&self) {
        let mut l = open_log();
        log!(l, "FocusOut");
    }

    async fn reset(&self) {
        let mut l = open_log();
        log!(l, "Reset");
    }

    async fn enable(&self) {
        let mut l = open_log();
        log!(l, "Enable");
    }

    async fn disable(&self) {
        let mut l = open_log();
        log!(l, "Disable");
    }

    async fn set_capabilities(&self, caps: u32) {
        let mut l = open_log();
        log!(l, "SetCapabilities caps={caps:#x}");
    }

    // --- Signals ---

    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Send a raw D-Bus signal on the IBus Engine interface.
///
/// This bypasses the zbus object-server interface machinery entirely,
/// avoiding potential deadlocks with the interface ref / object-server lock.
async fn send_engine_signal<B>(
    conn: &zbus::Connection,
    signal_name: &str,
    body: &B,
) -> zbus::Result<()>
where
    B: zbus::export::serde::ser::Serialize + zbus::zvariant::DynamicType,
{
    let mut l = open_log();
    log!(l, "  send_engine_signal: building {signal_name}...");
    let msg = zbus::message::Message::signal(
        "/org/freedesktop/IBus/Engine/SeagullIME",
        "org.freedesktop.IBus.Engine",
        signal_name,
    )?
    .build(body)?;
    log!(l, "  send_engine_signal: sending {signal_name}...");
    conn.send(&msg).await?;
    log!(l, "  send_engine_signal: {signal_name} sent OK");
    Ok(())
}

/// Helper to emit preedit update signals via raw D-Bus messages.
async fn emit_preedit(
    conn: &zbus::Connection,
    preedit: &str,
) -> zbus::Result<()> {
    if preedit.is_empty() {
        send_engine_signal(conn, "HidePreeditText", &()).await?;
        let text = ibus_text("");
        send_engine_signal(conn, "UpdatePreeditText", &(text, 0u32, false)).await?;
    } else {
        let text = ibus_text(preedit);
        let cursor_pos = preedit.len() as u32;
        send_engine_signal(conn, "UpdatePreeditText", &(text, cursor_pos, true)).await?;
        send_engine_signal(conn, "ShowPreeditText", &()).await?;
    }
    Ok(())
}

/// Process a stroke action and emit appropriate D-Bus signals.
pub async fn emit_for_action(
    action: &BufferAction,
    preedit: &str,
    conn: &zbus::Connection,
) -> zbus::Result<()> {
    let mut l = open_log();
    log!(l, "  emit_for_action: entered with action={action:?}");
    match action {
        BufferAction::Noop => {}
        BufferAction::UpdatePreedit | BufferAction::CommitAndPreedit => {
            emit_preedit(conn, preedit).await?;
        }
        BufferAction::FlushAndCommitAndPreedit { flushed } => {
            let commit = ibus_text(&format!("{flushed} "));
            send_engine_signal(conn, "CommitText", &(commit,)).await?;
            emit_preedit(conn, preedit).await?;
        }
    }

    Ok(())
}
