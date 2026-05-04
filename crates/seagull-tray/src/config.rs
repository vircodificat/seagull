use std::path::PathBuf;

use log::{info, warn};

#[derive(Debug, Clone)]
pub struct Config {
    pub device: DeviceConfig,
    pub dictionary_path: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub devices: Vec<String>,
    pub auto_detect: bool,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            auto_detect: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut config = Self {
            device: DeviceConfig::default(),
            dictionary_path: default_dictionary_path(),
            config_path: default_config_path(),
        };

        if let Some(path) = std::env::var("SEAGULL_CONFIG_PATH").ok() {
            config.config_path = PathBuf::from(path);
        }

        if config.config_path.exists() {
            info!("Loading config from: {:?}", config.config_path);
            if let Err(e) = config.load_from_file() {
                warn!("Failed to parse config {:?}: {e}", config.config_path);
            }
        } else {
            info!("No config file at {:?}, using defaults", config.config_path);
        }

        if let Ok(path) = std::env::var("SEAGULL_SERIAL_DEVICE") {
            config.device.devices = vec![path];
            config.device.auto_detect = false;
        }

        config
    }

    fn load_from_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(&self.config_path)?;
        let toml_value: toml::Value = contents.parse()?;

        if let Some(device) = toml_value.get("device") {
            if let Some(devices) = device.get("devices").and_then(|v| v.as_array()) {
                self.device.devices = devices
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
            if let Some(auto_detect) = device.get("auto_detect").and_then(|v| v.as_bool()) {
                self.device.auto_detect = auto_detect;
            }
        }

        if let Some(dictionary) = toml_value.get("dictionary") {
            if let Some(path) = dictionary.get("path").and_then(|v| v.as_str()) {
                self.dictionary_path = PathBuf::from(path);
            }
        }

        Ok(())
    }

    pub fn device_candidates(&self) -> Vec<String> {
        self.device.devices.clone()
    }

    pub fn log_dir_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".local/share/seagull-tray/")
    }
}

fn default_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config/seagull/ime.toml")
}

fn default_dictionary_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config/seagull/seagull.json")
}

#[allow(dead_code)]
fn auto_detect_devices() -> Vec<String> {
    let mut devices = Vec::new();
    let bases = ["/dev/serial/by-id", "/dev/serial/by-path", "/dev"];

    for base in bases {
        let path = std::path::Path::new(base);
        if !path.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            let mut entries: Vec<_> = entries.flatten().collect();
            if base == "/dev/serial/by-id" {
                entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            }

            for entry in entries {
                let p = entry.path();
                if p.is_file() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.contains("if02")
                        || name.contains("-interface-02")
                        || name.contains("USB")
                        || name.contains("serial")
                        || name.contains("ACM")
                    {
                        info!("Auto-detected candidate: {:?}", p);
                        devices.push(p.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    devices
}
