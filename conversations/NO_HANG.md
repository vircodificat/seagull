# Serial Device Disconnect Handling — Plan

## Problem Analysis

### Symptoms
1. Unplugging the serial device causes the IME to hang.
2. One of the threads spins at 100% CPU.

### Root Cause
**File:** `seagull/src/device/serial.rs`

The `read_stroke()` method loops indefinitely and silently ignores read errors:

```rust
loop {
    let buf_slice = &mut buf[total_amount..6];
    match self.port().read(buf_slice) {
        Ok(amount) => {
            total_amount += amount;
        },
        Err(_e) => {
            // Silently ignores errors — loops again immediately
        }
    }

    if total_amount == 6 {
        break;
    }
}
```

When the device is unplugged, every read call returns an error. The loop never breaks, never sleeps, and never terminates. This spins the thread at 100% CPU.

Additionally, `SerialDevice::new()` opens the device once and holds the open handle. If the device disappears, there is no mechanism to detect the disconnect or attempt reconnection.

### Architecture
The IME spawns a **single blocking thread** (`ime/src/main.rs`) that reads from the serial device and sends keycodes over a tokio channel:

```
[Serial Thread] ──read_stroke()──> [tokio MPSC channel] ──> [Async Stroke Loop]
```

The serial thread blocks on `device.read_stroke()`. When the device disappears, the thread spins in the error-handling loop. The async main task waits on the channel, which never receives data. The main task appears to hang (it's actually blocked waiting on the channel).

---

## Recommended Course of Action

### 1. Modify `SerialDevice` — Return Errors Instead of Silencing Them
**File:** `seagull/src/device/serial.rs`

Change `read_stroke()` to propagate read errors instead of silently ignoring them. This allows callers to handle device disconnects explicitly.

```rust
impl Device for SerialDevice {
    fn read_stroke(&mut self) -> Keycode {
        // ... existing code
    }

    // Add a fallible variant
    fn try_read_stroke(&mut self) -> Result<Keycode, serialport::Error> {
        let mut buf = [0; 6];
        let mut total_amount = 0;

        loop {
            let buf_slice = &mut buf[total_amount..6];
            let amount = self.port().read(buf_slice)?;
            total_amount += amount;

            if total_amount == 6 {
                break;
            }
        }
        // ... rest of parsing
    }
}
```

**Alternative:** Return a sentinel `Keycode` variant (e.g., `Keycode::disconnected()`) when a read fails, letting the caller handle it without changing the return type. However, propagating errors via `Result` is cleaner and more idiomatic.

### 2. Add `SerialDevice::try_reconnect()`
**File:** `seagull/src/device/serial.rs`

```rust
impl SerialDevice {
    pub fn new(device: &str) -> Result<SerialDevice, serialport::Error> {
        // ... existing code
    }

    pub fn try_reconnect(&mut self, device_path: &str) -> Result<bool, serialport::Error> {
        let new_port = serialport::new(device_path, 9600)
            .timeout(Duration::from_millis(10))
            .open()?;

        self.0 = new_port;
        Ok(true)
    }
}
```

### 3. Handle Disconnects in the Serial Reader Thread
**File:** `ime/src/main.rs`

Change the serial reader thread to catch read errors, transition to a disconnected state, poll every second for reconnection, and resume normal operation when the device reappears.

```rust
std::thread::spawn(move || {
    let mut device = match SerialDevice::new(&serial_device_path) {
        Ok(d) => d,
        Err(e) => {
            log!(serial_logger, "FATAL: Failed to open serial device: {e}");
            return;
        }
    };

    loop {
        match device.try_read_stroke() {
            Ok(keycode) => {
                let stroke = keycode.stroke();
                log!(serial_logger, "Stroke received: {stroke}");
                if tx.blocking_send(keycode).is_err() {
                    break;
                }
            }
            Err(e) => {
                log!(serial_logger, "Serial read error: {e}, device disconnected");

                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));

                    match device.try_reconnect(&serial_device_path) {
                        Ok(_) => {
                            log!(serial_logger, "Device reconnected");
                            break;
                        }
                        Err(e) => {
                            log!(serial_logger, "Still disconnected: {e}");
                        }
                    }
                }
            }
        }
    }
});
```

### 4. (Optional) Emit a D-Bus Signal for Disconnect/Reconnect Events
**File:** `ime/src/engine.rs`

Add a D-Bus property that clients can query to check if the device is connected.

---

## Summary of Changes

| File | Change |
|------|--------|
| `seagull/src/device/serial.rs` | Add `try_read_stroke()` that propagates errors |
| `seagull/src/device/serial.rs` | Add `try_reconnect()` method |
| `ime/src/main.rs` | Handle read errors in the serial thread |
| `ime/src/main.rs` | Implement 1-second polling for reconnection |
| `ime/src/main.rs` | Log disconnect and reconnect events |

---

## Testing

1. Start the IME with the serial device connected.
2. Unplug the serial device while typing strokes.
3. Verify that the IME thread does **not** spin at 100% CPU.
4. Verify that a disconnect message is logged.
5. Reconnect the device.
6. Verify that a reconnect message is logged.
7. Verify that strokes are received after reconnection.