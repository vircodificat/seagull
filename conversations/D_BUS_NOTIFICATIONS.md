# D-Bus Notifications Implementation

## Overview
Implemented D-Bus notifications to provide user feedback for critical events in the SeagullIME input method engine.

## Events Notified

### 1. Device Disconnected
- **When**: Serial device read fails (e.g., unplugged)
- **Urgency**: Critical (2)
- **Icon**: dialog-warning
- **Message**: "Steno Device Disconnected" - The serial device has been disconnected. Attempting to reconnect...

### 2. Device Reconnected
- **When**: Serial device successfully reconnects after disconnect
- **Urgency**: Normal (1)
- **Icon**: dialog-information
- **Message**: "Steno Device Reconnected" - The serial device is now connected.

### 3. Dictionary File Not Found
- **When**: Dictionary file fails to load at startup
- **Urgency**: Critical (2)
- **Icon**: dialog-error
- **Message**: "Dictionary File Not Found" - Shows the path that could not be loaded

## Implementation Details

### New Files
- **`ime/src/notifications.rs`**: Core notification module
  - `notify()`: Generic D-Bus notification function
  - `device_disconnected()`: Device disconnect notification
  - `device_reconnected()`: Device reconnect notification
  - `dictionary_not_found()`: Dictionary load error notification

### Modified Files
- **`ime/src/main.rs`**:
  - Added `mod notifications` import
  - Serial reader thread now sends notifications on disconnect/reconnect
  - Dictionary loading errors attempt to send notification before exit

## Technical Details

### D-Bus Interface
- Uses `org.freedesktop.Notifications` D-Bus service
- Calls `Notify` method on `/org/freedesktop/Notifications`
- Supports urgency levels via hints (0=low, 1=normal, 2=critical)
- Notifications auto-expire after 5 seconds

### Notification Delivery
- Device events are sent via tokio async tasks from the serial reader thread
- Dictionary error uses `tokio::spawn` for non-blocking notification attempt
- Best-effort delivery: If notification service is unavailable, application continues

## User Experience
Notifications appear in the system notification area/panel (GNOME, KDE, etc.) with:
- Clear summary of the issue
- Detailed body text
- Appropriate icon and urgency level
- Auto-dismissal after 5 seconds
