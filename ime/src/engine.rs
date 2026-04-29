use std::io::Write;
use std::sync::Arc;

use tokio::sync::Mutex;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, Value};
use zbus::interface;

use crate::buffer::{BufferAction, StrokeBuffer, SearchStateEnum};

pub type SharedConnection = Arc<Mutex<Option<zbus::Connection>>>;

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

/// Shared hint state across Engine and main loop
pub type SharedHintState = Arc<Mutex<bool>>;

/// Shared search state across Engine and main loop
pub type SharedSearchState = Arc<Mutex<SearchStateEnum>>;

/// IBus Engine — handles input method lifecycle and emits text signals.
#[derive(Clone)]
pub struct Engine {
    pub buffer: Arc<Mutex<StrokeBuffer>>,
    pub connection: SharedConnection,
    pub hint_showing: SharedHintState,
    pub search_state: SharedSearchState,
}

impl Engine {
    pub fn new(
        buffer: Arc<Mutex<StrokeBuffer>>,
        connection: SharedConnection,
        hint_showing: SharedHintState,
        search_state: SharedSearchState,
    ) -> Self {
        Self {
            buffer,
            connection,
            hint_showing,
            search_state,
        }
    }

    /// Show the "HINT" auxiliary text popup.
    pub async fn show_hint(&self) -> zbus::Result<()> {
        if let Some(conn) = self.connection.lock().await.as_ref() {
            emit_auxiliary_text(conn, "HINT").await?;
            *self.hint_showing.lock().await = true;
        }
        Ok(())
    }

    /// Hide the hint popup. Emits the hide signal unconditionally so the popup
    /// is dismissed even if our internal `hint_showing` flag is out of sync
    /// (e.g. when a keypress races with the show signal in `main.rs`). Callers
    /// must not invoke this while search mode is active, since search uses the
    /// same auxiliary popup.
    pub async fn hide_hint(&self) -> zbus::Result<()> {
        *self.hint_showing.lock().await = false;
        if let Some(conn) = self.connection.lock().await.as_ref() {
            hide_auxiliary_text(conn).await?;
        }
        Ok(())
    }

    /// Activate search mode
    pub async fn show_search(&self) -> zbus::Result<()> {
        let mut state = self.search_state.lock().await;
        *state = SearchStateEnum::Active(String::new());
        if let Some(conn) = self.connection.lock().await.as_ref() {
            emit_auxiliary_text(conn, "SEARCH: ").await?;
        }
        Ok(())
    }

    /// Hide search mode
    pub async fn hide_search(&self) -> zbus::Result<()> {
        let mut state = self.search_state.lock().await;
        *state = SearchStateEnum::Inactive;
        if let Some(conn) = self.connection.lock().await.as_ref() {
            hide_auxiliary_text(conn).await?;
        }
        Ok(())
    }

    /// Add a character to the search input
    pub async fn add_search_char(&self, ch: char) -> zbus::Result<()> {
        let mut state = self.search_state.lock().await;
        if let SearchStateEnum::Active(ref mut text) = *state {
            text.push(ch);
            if let Some(conn) = self.connection.lock().await.as_ref() {
                emit_auxiliary_text(conn, &format!("SEARCH: {}", text)).await?;
            }
        }
        Ok(())
    }

    /// Handle backspace in search mode (delete last character)
    pub async fn search_backspace(&self) -> zbus::Result<()> {
        let mut state = self.search_state.lock().await;
        if let SearchStateEnum::Active(ref mut text) = *state {
            text.pop();
            if let Some(conn) = self.connection.lock().await.as_ref() {
                emit_auxiliary_text(conn, &format!("SEARCH: {}", text)).await?;
            }
        }
        Ok(())
    }

    /// Perform dictionary lookup and show result
    pub async fn perform_lookup(&self, word: &str) -> zbus::Result<()> {
        let result = {
            let buf = self.buffer.lock().await;
            buf.reverse_lookup_word(word)
        };

        let display = if let Some(outline) = result {
            format!("{} {}", outline.extended(), word)
        } else {
            format!("NOT FOUND {}", word)
        };

        let mut state = self.search_state.lock().await;
        *state = SearchStateEnum::ShowingResult(word.to_string());

        if let Some(conn) = self.connection.lock().await.as_ref() {
            emit_auxiliary_text(conn, &display).await?;
        }
        Ok(())
    }

    /// Handle keyboard input when search mode is active
    async fn handle_search_key_event(&self, keyval: u32) -> bool {
        let mut l = open_log();

        // Escape key: keyval = 0xFF1B (close search without lookup)
        if keyval == 0xFF1B {
            log!(l, "  Escape pressed: closing search");
            if let Err(e) = self.hide_search().await {
                log!(l, "  WARNING: Failed to hide search: {e}");
            }
            return true; // Consumed
        }

        // Backspace key: keyval = 0xFF08
        if keyval == 0xFF08 {
            log!(l, "  Backspace pressed in search");
            if let Err(e) = self.search_backspace().await {
                log!(l, "  WARNING: Failed to handle backspace: {e}");
            }
            return true; // Consumed
        }

        // Enter key: keyval = 0xFF0D (perform lookup)
        if keyval == 0xFF0D {
            log!(l, "  Enter pressed in search");
            let word = {
                let state = self.search_state.lock().await;
                if let SearchStateEnum::Active(text) = &*state {
                    text.clone()
                } else {
                    String::new()
                }
            };
            if !word.is_empty() {
                if let Err(e) = self.perform_lookup(&word).await {
                    log!(l, "  WARNING: Failed to perform lookup: {e}");
                }
            }
            return true; // Consumed
        }

        // Convert keyval to character (only ASCII characters)
        if let Some(ch) = char::from_u32(keyval) {
            // Check if it's a valid search character
            if crate::buffer::is_search_key(ch) {
                log!(l, "  Adding character to search: '{}'", ch);
                if let Err(e) = self.add_search_char(ch).await {
                    log!(l, "  WARNING: Failed to add character: {e}");
                }
                return true; // Consumed
            }
        }

        // Unknown key in search mode - pass through
        log!(l, "  Unknown key in search mode (keyval=0x{:X}): passing through", keyval);
        false
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl Engine {
    /// IBus calls this for each key event (press and release).
    /// Returns true only if the IME consumed the key (preventing it from reaching
    /// the application or window manager). Returns false to pass the key through
    /// normally — this is the correct mechanism for pass-through, not ForwardKeyEvent.
    ///
    /// Steno input arrives via the serial device, not the keyboard, so keyboard
    /// events are only handled here to flush preedit when necessary.
    async fn process_key_event(
        &self,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> bool {
        let mut l = open_log();
        log!(l, "ProcessKeyEvent: keyval=0x{:X} keycode={} state=0x{:X}", keyval, keycode, state);

        // Ignore key release events — only act on key presses.
        // IBUS_RELEASE_MASK is bit 30.
        const IBUS_RELEASE_MASK: u32 = 1 << 30;
        if state & IBUS_RELEASE_MASK != 0 {
            log!(l, "  Key release — ignoring");
            return false;
        }

        // Check if search mode is active - handle keyboard input for search.
        // Don't call hide_hint here: search uses the same auxiliary popup, and
        // hint_showing is already false whenever search mode is active.
        {
            let search_state_lock = self.search_state.lock().await;
            if let SearchStateEnum::Active(_) = *search_state_lock {
                drop(search_state_lock); // Release lock before async calls
                return self.handle_search_key_event(keyval).await;
            }
        }

        // Any keyboard key dismisses the hint popup.
        if let Err(e) = self.hide_hint().await {
            log!(l, "  WARNING: Failed to hide hint: {e}");
        }

        // Get the connection, if available
        let conn = match self.connection.lock().await.as_ref() {
            Some(conn) => conn.clone(),
            None => {
                log!(l, "  WARNING: No D-Bus connection available");
                return false;
            }
        };

        // Backspace key: keyval = 0xFF08 (XK_BackSpace)
        let is_backspace = keyval == 0xFF08;

        // Get the current preedit
        let preedit = {
            let buf = self.buffer.lock().await;
            buf.preedit_string()
        };

        // If there's preedit text, handle it based on the key type
        if !preedit.is_empty() {
            if is_backspace {
                // Backspace with preedit: clear the preedit and consume the key.
                log!(l, "  Backspace pressed: clearing preedit '{}'", preedit);
                {
                    let mut buf = self.buffer.lock().await;
                    buf.clear();
                }
                if let Err(e) = emit_preedit(&conn, "").await {
                    log!(l, "  ERROR emitting preedit update: {e}");
                }
                // Consumed — don't let backspace reach the application.
                return true;
            } else {
                // Any other key with preedit: commit the preedit first, then let
                // the key pass through normally by returning false below.
                log!(l, "  Committing preedit before passing key through: '{}'", preedit);
                let commit = ibus_text(&format!("{} ", preedit));
                if let Err(e) = emit_commit_text(&conn, commit).await {
                    log!(l, "  ERROR emitting commit text: {e}");
                }
                {
                    let mut buf = self.buffer.lock().await;
                    buf.clear();
                }
                if let Err(e) = emit_preedit(&conn, "").await {
                    log!(l, "  ERROR emitting preedit update: {e}");
                }
            }
        }

        // Return false: we did not consume this key. IBus will pass it through to
        // the application and window manager via the normal system path. This is
        // the correct way to forward keys (including modifier combos like Alt+Tab),
        // as opposed to ForwardKeyEvent which routes through IBus and misses
        // window-manager shortcuts.
        log!(l, "  Passing key through (not consumed)");
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
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Build and send a D-Bus signal message on the IBus Engine interface.
fn engine_signal_builder<'a>(
    signal_name: &'a str,
) -> zbus::Result<zbus::message::Builder<'a>> {
    zbus::message::Message::signal(
        "/org/freedesktop/IBus/Engine/SeagullIME",
        "org.freedesktop.IBus.Engine",
        signal_name,
    )
}

/// Emit an empty signal (no body).
async fn emit_signal_empty(conn: &zbus::Connection, name: &str) -> zbus::Result<()> {
    let msg = engine_signal_builder(name)?.build(&())?;
    conn.send(&msg).await
}

/// Emit CommitText signal: body = `v` (one variant arg).
async fn emit_commit_text(conn: &zbus::Connection, text: Value<'_>) -> zbus::Result<()> {
    // Single variant arg: signature is just `v`
    let msg = engine_signal_builder("CommitText")?.build(&(text,))?;
    conn.send(&msg).await
}

/// Emit ForwardKeyEvent signal: body = `uuu` (keyval, keycode, state as flat args).
async fn emit_forward_key(conn: &zbus::Connection, keyval: u32, keycode: u32, state: u32) -> zbus::Result<()> {
    use zbus::zvariant;

    let ctxt0 = zvariant::serialized::Context::new_dbus(zvariant::LE, 0);
    let kv_bytes = zvariant::to_bytes(ctxt0, &keyval)?;

    let off1 = kv_bytes.bytes().len();
    let ctxt1 = zvariant::serialized::Context::new_dbus(zvariant::LE, off1);
    let kc_bytes = zvariant::to_bytes(ctxt1, &keycode)?;

    let off2 = off1 + kc_bytes.bytes().len();
    let ctxt2 = zvariant::serialized::Context::new_dbus(zvariant::LE, off2);
    let st_bytes = zvariant::to_bytes(ctxt2, &state)?;

    let mut body = Vec::new();
    body.extend_from_slice(kv_bytes.bytes());
    body.extend_from_slice(kc_bytes.bytes());
    body.extend_from_slice(st_bytes.bytes());

    let mut l = open_log();
    log!(l, "  ForwardKeyEvent: keyval=0x{:X} keycode={} state=0x{:X} body_hex={:02X?}",
         keyval, keycode, state, &body);

    let msg = unsafe {
        engine_signal_builder("ForwardKeyEvent")?
            .build_raw_body(&body, "uuu", vec![])?
    };
    log!(l, "  ForwardKeyEvent: msg signature={:?}", msg.header().signature());
    conn.send(&msg).await
}

/// Emit UpdateAuxiliaryText signal: body = `vb` (IBusText variant + visible bool).
async fn emit_update_auxiliary_text(
    conn: &zbus::Connection,
    text: Value<'_>,
    visible: bool,
) -> zbus::Result<()> {
    use zbus::zvariant;

    let ctxt0 = zvariant::serialized::Context::new_dbus(zvariant::LE, 0);
    let text_bytes = zvariant::to_bytes(ctxt0, &text)?;

    let off1 = text_bytes.bytes().len();
    let ctxt1 = zvariant::serialized::Context::new_dbus(zvariant::LE, off1);
    let vis_bytes = zvariant::to_bytes(ctxt1, &visible)?;

    let mut body = Vec::new();
    body.extend_from_slice(text_bytes.bytes());
    body.extend_from_slice(vis_bytes.bytes());

    let msg = unsafe {
        engine_signal_builder("UpdateAuxiliaryText")?
            .build_raw_body(&body, "vb", vec![])?
    };

    conn.send(&msg).await
}

/// Show an auxiliary text popup near the preedit with the given message.
pub async fn emit_auxiliary_text(conn: &zbus::Connection, hint: &str) -> zbus::Result<()> {
    let text = ibus_text(hint);
    emit_update_auxiliary_text(conn, text, true).await?;
    emit_signal_empty(conn, "ShowAuxiliaryText").await?;
    Ok(())
}

/// Hide the auxiliary text popup.
pub async fn hide_auxiliary_text(conn: &zbus::Connection) -> zbus::Result<()> {
    emit_signal_empty(conn, "HideAuxiliaryText").await?;
    let text = ibus_text("");
    emit_update_auxiliary_text(conn, text, false).await?;
    Ok(())
}

/// Emit UpdatePreeditText signal: body = `vubu` (four separate args).
///
/// IBus expects: text (variant), cursor_pos (u32), visible (bool), mode (u32).
/// Mode: 0 = IBUS_ENGINE_PREEDIT_CLEAR, 1 = IBUS_ENGINE_PREEDIT_COMMIT.
///
/// We use `build_raw_body` because a Rust tuple serializes as a D-Bus struct
/// `(vubu)` rather than flat args `vubu`.
async fn emit_update_preedit(
    conn: &zbus::Connection,
    text: Value<'_>,
    cursor_pos: u32,
    visible: bool,
) -> zbus::Result<()> {
    use zbus::zvariant;

    let mode: u32 = 0; // IBUS_ENGINE_PREEDIT_CLEAR

    // Serialize each arg at the correct body offset so alignment/padding is right.
    let ctxt0 = zvariant::serialized::Context::new_dbus(zvariant::LE, 0);
    let text_bytes = zvariant::to_bytes(ctxt0, &text)?;

    let off1 = text_bytes.bytes().len();
    let ctxt1 = zvariant::serialized::Context::new_dbus(zvariant::LE, off1);
    let pos_bytes = zvariant::to_bytes(ctxt1, &cursor_pos)?;

    let off2 = off1 + pos_bytes.bytes().len();
    let ctxt2 = zvariant::serialized::Context::new_dbus(zvariant::LE, off2);
    let vis_bytes = zvariant::to_bytes(ctxt2, &visible)?;

    let off3 = off2 + vis_bytes.bytes().len();
    let ctxt3 = zvariant::serialized::Context::new_dbus(zvariant::LE, off3);
    let mode_bytes = zvariant::to_bytes(ctxt3, &mode)?;

    let mut body = Vec::new();
    body.extend_from_slice(text_bytes.bytes());
    body.extend_from_slice(pos_bytes.bytes());
    body.extend_from_slice(vis_bytes.bytes());
    body.extend_from_slice(mode_bytes.bytes());

    let msg = unsafe {
        engine_signal_builder("UpdatePreeditText")?
            .build_raw_body(&body, "vubu", vec![])?
    };

    let mut l = open_log();
    log!(l, "  UpdatePreeditText signature={:?} body_len={}", msg.header().signature(), body.len());
    conn.send(&msg).await
}

/// Helper to emit preedit update signals via raw D-Bus messages.
async fn emit_preedit(
    conn: &zbus::Connection,
    preedit: &str,
) -> zbus::Result<()> {
    if preedit.is_empty() {
        emit_signal_empty(conn, "HidePreeditText").await?;
        let text = ibus_text("");
        emit_update_preedit(conn, text, 0, false).await?;
    } else {
        let text = ibus_text(preedit);
        let cursor_pos = preedit.len() as u32;
        emit_update_preedit(conn, text, cursor_pos, true).await?;
        emit_signal_empty(conn, "ShowPreeditText").await?;
    }
    Ok(())
}

/// Process a stroke action and emit appropriate D-Bus signals.
pub async fn emit_for_action(
    action: &BufferAction,
    preedit: &str,
    conn: &zbus::Connection,
) -> zbus::Result<()> {
    match action {
        BufferAction::Noop => {}
        BufferAction::UpdatePreedit | BufferAction::CommitAndPreedit => {
            emit_preedit(conn, preedit).await?;
        }
        BufferAction::FlushAll { flushed } => {
            let commit = ibus_text(&format!("{flushed} "));
            emit_commit_text(conn, commit).await?;
            emit_preedit(conn, "").await?;
        }
        BufferAction::SendEnter => {
            let enter_keyval: u32 = 0xFF0D;
            let enter_keycode: u32 = 28; // EVDEV KEY_ENTER
            let enter_state: u32 = 0;
            let release_state: u32 = 1 << 30; // IBUS_RELEASE_MASK
            emit_forward_key(conn, enter_keyval, enter_keycode, enter_state).await?;
            emit_forward_key(conn, enter_keyval, enter_keycode, release_state).await?;
        }
        BufferAction::SendBackspace => {
            let bs_keyval: u32 = 0xFF08;
            let bs_keycode: u32 = 14; // EVDEV KEY_BACKSPACE
            let bs_state: u32 = 0;
            let release_state: u32 = 1 << 30; // IBUS_RELEASE_MASK
            emit_forward_key(conn, bs_keyval, bs_keycode, bs_state).await?;
            emit_forward_key(conn, bs_keyval, bs_keycode, release_state).await?;
        }
    }

    Ok(())
}
