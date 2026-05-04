# Seagull Tray — Implementation Plan

## Goal
Add a system-tray app for the Seagull IME. The tray:
- Shows whether the steno serial device is connected (two icons, switched at runtime).
- On click, presents a menu: open dictionary file, open settings, open log file, and toggle Connect/Disconnect.

## Workspace restructuring

Two restructurings happen up front, before any feature work:

1. The existing `ime/` directory is renamed to `seagull-ime/` so all crate directory names match their Cargo `package.name`.
2. All Rust crates are moved under a new top-level `crates/` directory, so the repo root is no longer cluttered with per-crate folders sitting next to `data/`, `theory/`, `scripts/`, etc.

After this work, the repo's Rust crates live at:

```
crates/
  seagull/        # core library + CLI
  seagull-py/     # PyO3 / maturin bindings
  seagull-ime/    # IBus engine (renamed from ime/)
  seagull-tray/   # NEW — system tray app
  seagull-ipc/    # NEW — shared D-Bus message types
```

The workspace's `target/` directory continues to live at the repo root (next to the root `Cargo.toml`), as Cargo always places it next to the workspace manifest.

### Concrete restructuring steps (their own commit-sized chunk)

1. `git mv ime seagull-ime` (preserves history for the rename).
2. `mkdir crates && git mv seagull seagull-py seagull-ime crates/` — move all three existing crates under `crates/`. The two new crates (`seagull-tray`, `seagull-ipc`) are then *created* directly under `crates/`.
3. Update root `Cargo.toml` `workspace.members` to:
   ```toml
   members = [
       "crates/seagull",
       "crates/seagull-py",
       "crates/seagull-ime",
       "crates/seagull-tray",
       "crates/seagull-ipc",
   ]
   ```
4. Update root `Makefile`: the `ime:` target's `$(MAKE) -C ime build` becomes `$(MAKE) -C crates/seagull-ime build`. (The Make target name stays `ime` for muscle memory.) Add a new `tray:` target invoking `$(MAKE) -C crates/seagull-tray build`.
5. Update `pyproject.toml`: `[tool.maturin] manifest-path = "seagull-py/Cargo.toml"` → `"crates/seagull-py/Cargo.toml"`.
6. Inter-crate path deps (`seagull = { path = "../seagull" }` in `seagull-py/Cargo.toml`) are unchanged — both crates move together so the relative path is the same.
7. Per-crate Makefiles: `seagull-ime/Makefile` references `../target/release/seagull-ime`. Because the crate is now one level deeper (`crates/seagull-ime/`), this becomes `../../target/release/seagull-ime`. Same adjustment for the new `seagull-tray/Makefile`.
8. Update `seagull-ime/README.md`: replace `make -C ime …` with `make -C crates/seagull-ime …`.
9. Verify with `cargo build --workspace` and `uv run maturin develop` that no other code or build script referenced the old paths.

## Process roles (revised)
The serial device is owned **entirely by the tray**. Two binaries:

- `seagull-tray` (new): owns the serial port. Reads 6-byte frames, decodes them into `(Stroke, is_control)`, and forwards each stroke to the IME via D-Bus. Owns the tray icon, menu, and Connect/Disconnect state.
- `seagull-ime` (existing, slimmed down): loses all serial-port code. Only loads the dictionary, runs the stroke buffer, drives the IBus engine, and exposes one new D-Bus method to receive strokes from the tray.

There is **no auto-reconnect** anywhere. Connect/Disconnect is fully user-driven via the tray.

## IPC: D-Bus carrying a Rust-style message enum

The two binaries communicate over the **session D-Bus**. We use D-Bus (rather than a custom Unix socket) because the IME is already wired into D-Bus for ibus-daemon and notifications, and we get free service discovery and lifecycle tracking (the tray learns when `seagull-ime` appears/disappears via `org.freedesktop.DBus.NameOwnerChanged`).

On top of D-Bus we use a **single Rust enum** as the message type rather than one D-Bus method per message kind. This keeps the surface small, gives us `match`-exhaustiveness, and makes adding new messages a one-liner on each side.

### Shared message type (lives in a new tiny `seagull-ipc` crate)

```rust
// crates/seagull-ipc/src/lib.rs
use serde::{Serialize, Deserialize};
use zvariant::Type;

#[derive(Serialize, Deserialize, Type, Debug, Clone)]
pub enum ImeMessage {
    /// A stroke read from the steno device.
    Stroke { bits: u32, is_control: bool },
    // future variants go here, e.g.:
    // Ping,
    // ReloadDictionary,
    // FlushBuffer,
}
```

We put this in a new `seagull-ipc` workspace crate (deps: `serde`, `zvariant`) rather than in the existing `seagull` crate, because the core `seagull` crate is also consumed by `seagull-py` and the CLI; we don't want D-Bus / zvariant deps to leak into those. Both `seagull-ime` and `seagull-tray` will depend on `seagull-ipc`.

### D-Bus surface (IME side)
- Bus name: `org.seagull.IME`
- Object path: `/org/seagull/IME`
- Interface: `org.seagull.IME.Control`
- Method: `Send(msg: ImeMessage) -> ()` — fire-and-forget; the tray does not await processing.

zvariant serializes Rust enums to D-Bus as a tagged `(uv)` (discriminant + variant payload), so the enum round-trips losslessly. Adding a variant later is one line in `ImeMessage` plus one new `match` arm in the IME's `Send` handler.

### Tray side
- The tray uses a zbus `#[proxy]`-derived client for `org.seagull.IME.Control` and calls `proxy.send(&ImeMessage::Stroke { ... }).await`.
- The tray watches `NameOwnerChanged` so it knows whether the IME is currently on the bus. If it isn't, calls to `Send` are skipped (and the stroke is dropped with a log line) rather than blocking or erroring noisily on every keypress.
- The tray does **not** expose any D-Bus interface itself — Connect/Disconnect/status are internal tray state only.

## File layout

The two new crates live under `crates/` alongside the moved/renamed existing ones:

```
crates/
  seagull-tray/
    Cargo.toml
    Makefile
    seagull-tray.desktop
    src/
      main.rs               # tokio entry, ksni service registration, wires everything together
      tray.rs               # `ksni::Tray` impl: holds shared state, builds menu, handles activate
      serial.rs             # serial reader thread, connect/disconnect, stroke forwarding
      ime_client.rs         # D-Bus proxy: ImeMessage sender
      config.rs             # reads ~/.config/seagull/ime.toml ([device] section)
      actions.rs            # high-level menu actions (open dict/settings/log)
      platform/
        mod.rs              # re-exports the active platform's `Opener` impl
        linux.rs            # Linux Opener: spawns `xdg-open`
        // (macos.rs, windows.rs added later)

  seagull-ipc/
    Cargo.toml
    src/
      lib.rs                # ImeMessage enum, derives Serialize/Deserialize/Type
```

Both crates are added to the root `Cargo.toml` workspace members (as `"crates/seagull-tray"` and `"crates/seagull-ipc"`) alongside the now-relocated `crates/seagull-ime`.

## Dependencies (tray crate)
- `ksni` — chosen tray backend. Pure Rust; speaks the StatusNotifierItem protocol directly over D-Bus (via zbus), so no `libxdo` / `libayatana-appindicator` system packages are required. `ksni` accepts theme icon names directly via `Tray::icon_name`, so no separate icon-resolution dep is needed.
- `zbus` — D-Bus client (already used in the workspace).
- `tokio` — async runtime (already used).
- `serialport` (transitively via `seagull` crate's serial module — we'll reuse `seagull::device::serial::SerialDevice` and `seagull::device::Device::read_stroke`).
- `toml`, `log`, `simplelog` — same as IME, for config + logging.
- No external "open" crate. The platform abstraction below uses `std::process::Command` directly so we keep dependency surface small and explicit.

## Platform abstraction (file/URL opening)

To make later retargeting (macOS, Windows) straightforward, all "open this thing in the user's default handler" calls go through a single trait, not direct `xdg-open` invocations from menu code.

```rust
// crates/seagull-tray/src/platform/mod.rs
use std::path::Path;

pub trait Opener: Send + Sync {
    /// Open a local file or directory in the user's default handler.
    fn open_path(&self, path: &Path) -> std::io::Result<()>;
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxOpener as DefaultOpener;

// Future:
// #[cfg(target_os = "macos")]   mod macos;   pub use macos::MacOpener as DefaultOpener;
// #[cfg(target_os = "windows")] mod windows; pub use windows::WindowsOpener as DefaultOpener;
```

```rust
// crates/seagull-tray/src/platform/linux.rs
pub struct LinuxOpener;

impl super::Opener for LinuxOpener {
    fn open_path(&self, path: &Path) -> std::io::Result<()> {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
    }
}
```

Menu-action code in `actions.rs` accepts `&dyn Opener` (or holds an `Arc<dyn Opener>`) so it never references `xdg-open` or any other platform tool directly. The `main` function constructs `DefaultOpener` once at startup. Adding a new OS later is a matter of dropping in `platform/macos.rs` (typically `Command::new("open")`) or `platform/windows.rs` (typically `Command::new("cmd").args(["/C", "start", "", path])` or `ShellExecuteW`) — no changes to call sites.

## Config

`~/.config/seagull/ime.toml` is shared. Both binaries read it independently.
- `[device]` (`devices`, `auto_detect`) → used by `seagull-tray`.
- `[dictionary]`, `[buffer]` → used by `seagull-ime`.

`Settings` menu: opens `~/.config/seagull/ime.toml` through the `Opener` abstraction. If the file is missing, the tray writes a default template (mirroring the example in `crates/seagull-ime/README.md`) before calling `open_path`.

## `seagull-ime` changes

Removed:
- All references to `SerialDevice` in `crates/seagull-ime/src/main.rs`.
- The pre-startup serial-connect probe + `"FATAL: Failed to connect to any serial device"` exit.
- The serial reader thread and its inner reconnect loop (lines ~174–236 of current `main.rs`).
- The `NotificationEvent::DeviceDisconnected` / `DeviceReconnected` channel and the secondary `notif_connection`. (Disconnect notifications can be added back later from the tray side.)

Added:
- A new `Control` D-Bus interface implementation (handling `ImeMessage`), `serve_at("/org/seagull/IME", ...)` alongside the existing IBus Factory/Engine objects, claiming the `org.seagull.IME` bus name.
- A small mpsc channel (`tx`, `rx`) wired from the `Control::send` handler to the existing main `select!` loop. The select arm that processes strokes is unchanged in behavior.

Notes:
- We claim two bus names from the same `zbus::Connection`: `org.freedesktop.IBus.SeagullIME` (existing) and `org.seagull.IME` (new) using `Builder::name(...)` chained twice, or via `RequestName` after build.
- The `[device]` section in `Config` becomes unused on the IME side. I'll leave the parsing code intact (it's harmless) so the shared config struct stays simple, and only stop *using* the device fields.

## `seagull-tray` behavior

Startup:
1. Init logging to `~/.local/share/seagull-tray/seagull-tray.log`.
2. Load config.
3. Build the tray icon (initially "disconnected" theme icon) and menu.
4. Run the tray event loop on the main thread; the D-Bus + serial threads run on tokio.

Connect (menu click or, optional, on-startup-once — **not** on startup per spec):
- Iterate `device.device_candidates()`; for each, try `SerialDevice::new`. First success wins.
- Spawn a reader thread that loops on `read_stroke` and pushes each `Keycode` to a tokio channel.
- Tokio task drains the channel and calls `proxy.send(ImeMessage::Stroke { ... })` via a zbus proxy. If the call errors (IME not present), log and continue.
- Update icon to "connected".

Disconnect (menu click):
- Signal the reader thread to stop (atomic `should_stop` flag, joined after it returns from a read or errors out).
- Drop the `SerialDevice`.
- Update icon to "disconnected".

Read errors mid-session:
- Reader thread exits, tray flips to disconnected, log entry. No retry.

Menu rebuild:
- Each Connect/Disconnect transition rebuilds the menu so the right label is shown (`tray-icon` doesn't re-render dynamic labels nicely otherwise).

## Build / install

`crates/seagull-tray/Makefile`:
```
build:    cargo build --release -p seagull-tray
install:  install -Dm755 ../../target/release/seagull-tray $(DESTDIR)/usr/bin/seagull-tray
          install -Dm644 seagull-tray.desktop $(DESTDIR)/etc/xdg/autostart/seagull-tray.desktop
```
Note the `../../target/...` path: the workspace's `target/` lives at the repo root, two levels above `crates/seagull-tray/`. The same adjustment is made to the existing `crates/seagull-ime/Makefile` (`../target/...` → `../../target/...`) as part of the restructuring step.

`seagull-tray.desktop`:
```
[Desktop Entry]
Type=Application
Name=Seagull Tray
Exec=/usr/bin/seagull-tray
X-GNOME-Autostart-enabled=true
NoDisplay=false
```

Top-level `Makefile`:
- Update the existing `ime:` target's recipe from `$(MAKE) -C ime build` to `$(MAKE) -C crates/seagull-ime build`.
- Add a parallel `tray:` target invoking `$(MAKE) -C crates/seagull-tray build`.

## Testing

- Unit tests where they make sense (config parsing for `[device]`, icon-name resolution fallback).
- Manual smoke test:
  1. Start `seagull-ime` (via ibus-daemon).
  2. Start `seagull-tray`. Verify icon shows disconnected.
  3. Click Connect. Verify icon flips and strokes type into the focused app.
  4. Click Disconnect. Verify port releases (e.g. `lsof /dev/ttyACM0` shows nothing) and strokes stop.
  5. Click each Open menu item; verify `xdg-open` opens the right file.
  6. Stop the IME; verify Connect still succeeds but strokes are dropped with a log line; restart IME and verify strokes resume.

## Risks / open items

- **GNOME without AppIndicator extension shows nothing.** The `tray-icon` crate uses StatusNotifierItem on Linux. If the user is on stock GNOME, they need the AppIndicator/KStatusNotifierItem Support extension installed. (Same constraint applies to any SNI-based approach.)
- **`freedesktop-icons` lookup may fail** on minimal systems; I'll fall back to a built-in tiny PNG (red/green dot) if name resolution returns no path, just so the tray is always visible.
- **Bus-name claim ordering**: the IME currently uses `Builder::name("org.freedesktop.IBus.SeagullIME")`. Adding a second name needs either a second `Builder::name` chain (zbus 5 supports this) or an explicit `request_name` call after build. I'll verify against zbus 5 docs during implementation.
