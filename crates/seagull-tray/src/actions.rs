use std::path::Path;
use std::sync::Arc;

use log::{error, info, warn};

use crate::config::Config;
use crate::platform::Opener;

const DEFAULT_CONFIG_TEMPLATE: &str = r#"[device]
# List of serial device paths to try (in order). If empty and auto_detect is
# true, the tray will scan for available devices.
devices = []

# Enable auto-detection of serial devices on Connect.
auto_detect = true

[dictionary]
# Path to the Seagull JSON dictionary used by the IME.
path = "~/.config/seagull/seagull.json"

[buffer]
# Maximum number of words to buffer before flushing to the application.
max_size = 5
"#;

pub fn open_dictionary(opener: &dyn Opener, config: &Config) {
    open_with_log(opener, &config.dictionary_path, "dictionary");
}

pub fn open_settings(opener: &dyn Opener, config: &Config) {
    let path = &config.config_path;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                error!("Failed to create config directory {:?}: {e}", parent);
                return;
            }
        }
        match std::fs::write(path, DEFAULT_CONFIG_TEMPLATE) {
            Ok(()) => info!("Wrote default config template to {:?}", path),
            Err(e) => {
                error!("Failed to write default config to {:?}: {e}", path);
                return;
            }
        }
    }
    open_with_log(opener, path, "settings");
}

pub fn open_log(opener: &dyn Opener) {
    let path = Config::ime_log_path();
    if !path.exists() {
        warn!("IME log not found at {:?}; opening anyway", path);
    }
    open_with_log(opener, &path, "log");
}

fn open_with_log(opener: &dyn Opener, path: &Path, label: &str) {
    info!("Opening {label}: {:?}", path);
    if let Err(e) = opener.open_path(path) {
        error!("Failed to open {label} {:?}: {e}", path);
    }
}

/// Convenience wrapper used by the tray menu callbacks: spawns the open
/// action on a blocking task so the tray loop is never delayed by `xdg-open`.
pub fn spawn_open<F>(opener: Arc<dyn Opener>, f: F)
where
    F: FnOnce(&dyn Opener) + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        f(opener.as_ref());
    });
}
