use std::collections::HashMap;
use std::io::Write;
use zbus::Connection;
use zbus::zvariant::Value;

fn log_notif(msg: &str) {
    let log_dir = std::env::var("HOME")
        .map(|h| format!("{h}/.local/share/seagull-ime"))
        .unwrap_or_else(|_| "/tmp".to_string());
    let log_path = format!("{log_dir}/seagull-ime.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(f, "[{now}] NOTIF: {msg}");
    }
}

/// Send a D-Bus notification to the system notification daemon.
pub async fn notify(
    connection: &Connection,
    summary: &str,
    body: &str,
    icon: &str,
    urgency: u8, // 0=low, 1=normal, 2=critical
) -> zbus::Result<()> {
    log_notif(&format!("notify() called: summary={}", summary));

    let mut hints: HashMap<String, Value<'static>> = HashMap::new();
    hints.insert(
        "urgency".to_string(),
        Value::new(urgency),
    );

    let msg = zbus::message::Message::method_call(
        "/org/freedesktop/Notifications",
        "Notify",
    )?
    .destination("org.freedesktop.Notifications")?
    .interface("org.freedesktop.Notifications")?
    .build(&(
        "SeagullIME",           // app_name
        0u32,                   // replaces_id (0 = new notification)
        icon,                   // icon
        summary,                // summary
        body,                   // body
        vec![] as Vec<String>,  // actions
        hints,                  // hints
        5000i32,                // expire_timeout (ms)
    ))?;

    log_notif(&format!("Sending notification message, connection type: {:?}", connection.unique_name()));
    match connection.send(&msg).await {
        Ok(_) => {
            log_notif(&format!("Notification sent successfully: {}", summary));
            Ok(())
        }
        Err(e) => {
            log_notif(&format!("Failed to send notification: {}", e));
            Err(e)
        }
    }
}

/// Notify that the serial device has been disconnected.
pub async fn device_disconnected(connection: &Connection) -> zbus::Result<()> {
    notify(
        connection,
        "Steno Device Disconnected",
        "The serial device has been disconnected. Attempting to reconnect...",
        "dialog-warning",
        2, // critical
    )
    .await
}

/// Notify that the serial device has been reconnected.
pub async fn device_reconnected(connection: &Connection) -> zbus::Result<()> {
    notify(
        connection,
        "Steno Device Reconnected",
        "The serial device is now connected.",
        "dialog-information",
        1, // normal
    )
    .await
}

/// Notify that the dictionary file could not be found.
pub async fn dictionary_not_found(connection: &Connection, path: &str) -> zbus::Result<()> {
    notify(
        connection,
        "Dictionary File Not Found",
        &format!("Could not load dictionary from: {}", path),
        "dialog-error",
        2, // critical
    )
    .await
}
