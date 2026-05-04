# Seagull IME

A Linux IBus input method engine for stenography. Receives steno strokes from a
serial device, buffers them, looks up translations in the Seagull dictionary,
and outputs text via IBus preedit/commit over D-Bus.

## Prerequisites

- Rust toolchain (edition 2024)
- IBus framework (`ibus` package)
- A serial steno device (e.g. Lets Split v2)

## Build

```sh
make ime
# or directly:
make -C ime build
```

## Install

Build first as your normal user, then install with sudo:

```sh
make -C ime build
sudo make -C ime install
```

This copies the binary to `/usr/libexec/seagull-ime`, the IBus component XML to
`/usr/share/ibus/component/`, and refreshes the IBus cache.

## Enable in GNOME

GNOME Settings does not list custom IBus engines in its GUI. Add the input
source from the command line:

```sh
# Append seagull-ime to your existing input sources
CURRENT=$(gsettings get org.gnome.desktop.input-sources sources)
# e.g. if current is [('xkb', 'us')]:
gsettings set org.gnome.desktop.input-sources sources "[('xkb', 'us'), ('ibus', 'seagull-ime')]"
```

Then switch to **Seagull Steno** from the input source indicator in the top bar.

## Usage

- Type on your steno device — strokes appear in the preedit popup
- When strokes match a dictionary entry, the word is committed to the preedit
- Older words are flushed to the application as the buffer fills
- Press `*` (star) to undo the last stroke or decompose the last word

## Configuration

The IME can be configured via a TOML config file at `~/.config/seagull/ime.toml`.

### Config File Format

```toml
[device]
# List of serial device paths to try (in order). If empty and auto_detect is true,
# the IME will scan for available devices.
devices = [
    "/dev/serial/by-id/usb-Wootpatoot_Lets_Split_v2-if02",
    "/dev/ttyUSB0",
]

# Enable auto-detection of serial devices on startup and reconnection.
# When true, scans /dev/serial/by-id, /dev/serial/by-path, and common tty devices.
# Default: true
auto_detect = true

[dictionary]
# Path to the Seagull JSON dictionary
path = "~/.config/seagull/seagull.json"

[buffer]
# Maximum number of words to buffer before flushing to the application
max_size = 5
```

### Environment Variables (Override Config File)

Environment variables take precedence over the config file:

| Variable | Description |
|----------|-------------|
| `SEAGULL_CONFIG_PATH` | Path to a custom config file |
| `SEAGULL_SERIAL_DEVICE` | Serial device path (disables auto-detect when set) |
| `SEAGULL_DICTIONARY_PATH` | Path to the Seagull JSON dictionary |
| `SEAGULL_BUFFER_SIZE` | Max committed words before flushing |

## Uninstall

```sh
sudo make -C ime uninstall
```
