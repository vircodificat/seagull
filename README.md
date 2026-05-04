# Seagull Steno Suite

A stenography toolkit for Linux, organised as a Cargo workspace under `crates/`:

| Crate | Role |
|---|---|
| `seagull` | Core library: strokes, outlines, dictionary, prefix tree |
| `seagull-py` | Python bindings (via PyO3 / maturin) |
| `seagull-ipc` | Shared `ImeMessage` enum used over D-Bus |
| `seagull-ime` | IBus input method engine (the binary IBus launches) |
| `seagull-tray` | System tray icon; owns the serial steno device and forwards strokes to the IME over D-Bus |

The tray and IME are two separate processes. The tray reads from the serial
device and sends strokes to the IME via the `at.vircodific.seagull.IME.Control` D-Bus
interface. The IME does no serial I/O of its own.

## Prerequisites

- Rust toolchain (edition 2024)
- IBus framework (`ibus` package)
- A serial steno device (e.g. Lets Split v2, StenoKeyboards Polyglot)
- `xdg-utils` (the tray uses `xdg-open` for menu actions)

## Build

The IME and tray are treated as a single product. From the workspace root:

```sh
make ime
```

This builds both `target/release/seagull-ime` and `target/release/seagull-tray`.

## Install

Build first as your normal user, then install with sudo:

```sh
make ime
sudo make install
```

`make install` does not trigger a build, so cargo never runs as root.

What gets installed:

| Component | Path |
|---|---|
| IME binary | `/usr/libexec/seagull-ime` |
| IBus component descriptor | `/usr/share/ibus/component/seagull-ime.xml` |
| Tray binary | `/usr/bin/seagull-tray` |
| Tray autostart entry | `/etc/xdg/autostart/seagull-tray.desktop` |

The IME install also runs `ibus write-cache` so IBus picks up the new engine.

Uninstall:

```sh
sudo make uninstall
```

## Enable the IME in GNOME

GNOME Settings does not list custom IBus engines in its GUI. Add the input
source from the command line:

```sh
gsettings set org.gnome.desktop.input-sources sources \
    "[('xkb', 'us'), ('ibus', 'seagull-ime')]"
ibus restart
```

Then pick **Seagull Steno** from the input source indicator in the top bar.
IBus launches `/usr/libexec/seagull-ime` on demand — you do not start it
manually.

## Run the tray

The tray autostarts on your next desktop session. To start it now without
logging out:

```sh
seagull-tray &
```

The tray icon's menu lets you open the dictionary, settings, or log file,
toggle the device connection, and quit. Logs are written to
`~/.local/state/seagull/seagull-tray.log`.

## Configuration

See `crates/seagull-ime/README.md` for the full TOML config schema at
`~/.config/seagull/ime.toml` (covers device candidates, dictionary path, and
buffer size).

## Development

For iterating without going through a full install each time:

```sh
make ime
sudo install -m755 target/release/seagull-ime /usr/libexec/seagull-ime && ibus restart
pkill seagull-tray; ./target/release/seagull-tray &
```

Run tests:

```sh
cargo test -p seagull -p seagull-ipc -p seagull-tray
```
