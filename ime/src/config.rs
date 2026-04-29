use std::io::Write;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Config {
    pub device: DeviceConfig,
    pub dictionary: DictionaryConfig,
    pub buffer: BufferConfig,
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

#[derive(Debug, Clone)]
pub struct DictionaryConfig {
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct BufferConfig {
    pub max_size: usize,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            path: None,
            auto_detect: true,
        }
    }
}

impl Default for DictionaryConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            path: format!("{}/.config/seagull/seagull.json", home),
        }
    }
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self { max_size: 5 }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device: DeviceConfig::default(),
            dictionary: DictionaryConfig::default(),
            buffer: BufferConfig::default(),
        }
    }
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

macro_rules! log {
    ($logger:expr, $($arg:tt)*) => {{
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!($logger, "[{now}] CONFIG: {}", format!($($arg)*));
        let _ = $logger.flush();
    }};
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Config::default();
        let mut logger = open_log();

        if let Some(config_path) = std::env::var("SEAGULL_CONFIG_PATH").ok() {
            log!(logger, "Loading config from SEAGULL_CONFIG_PATH: {}", config_path);
            config.load_from_file(&config_path)?;
        } else {
            let default_path = Self::default_config_path();
            if default_path.exists() {
                log!(logger, "Loading config from: {:?}", default_path);
                config.load_from_file(&default_path)?;
            } else {
                log!(logger, "No config file found, using defaults");
            }
        }

        config.apply_env_overrides();
        config.validate()?;

        log!(logger, "Final config: {:?}", config);
        Ok(config)
    }

    fn default_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".config/seagull/ime.toml")
    }

    fn load_from_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
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
                self.dictionary.path = path.to_string();
            }
        }

        if let Some(buffer) = toml_value.get("buffer") {
            if let Some(max_size) = buffer.get("max_size").and_then(|v| v.as_integer()) {
                self.buffer.max_size = max_size as usize;
            }
        }

        Ok(())
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(path) = std::env::var("SEAGULL_SERIAL_DEVICE") {
            self.device.devices = vec![path];
            self.device.auto_detect = false;
        }
        if let Ok(path) = std::env::var("SEAGULL_DICTIONARY_PATH") {
            self.dictionary.path = path;
        }
        if let Ok(size) = std::env::var("SEAGULL_BUFFER_SIZE") {
            if let Ok(size) = size.parse() {
                self.buffer.max_size = size;
            }
        }
    }

    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.buffer.max_size == 0 {
            return Err("buffer.max_size must be greater than 0".into());
        }
        Ok(())
    }

    pub fn device_candidates(&self) -> Vec<String> {
        let mut candidates = self.device.devices.clone();
        if self.device.auto_detect {
            candidates.extend(Self::auto_detect_devices());
        }
        candidates
    }

    pub fn try_connect_device(path: &str) -> Option<String> {
        let mut logger = open_log();
        match seagull::device::serial::SerialDevice::new(path) {
            Ok(d) => {
                log!(logger, "Successfully connected to device: {}", path);
                drop(d);
                Some(path.to_string())
            }
            Err(e) => {
                log!(logger, "Failed to connect to {}: {}", path, e);
                None
            }
        }
    }

    fn auto_detect_devices() -> Vec<String> {
        let mut devices = Vec::new();
        let mut logger = open_log();

        let candidates = [
            "/dev/serial/by-id",
            "/dev/serial/by-path",
            "/dev",
        ];

        for base in candidates {
            let path = std::path::Path::new(base);
            if !path.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(path) {
                let mut entries: Vec<_> = entries.flatten().collect();

                if base == "/dev/serial/by-id" {
                    entries.sort_by(|a, b| {
                        let a_name = a.file_name();
                        let b_name = b.file_name();
                        b_name.cmp(&a_name)
                    });
                }

                for entry in entries {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Ok(subentries) = std::fs::read_dir(&path) {
                            for subentry in subentries.flatten() {
                                let subpath = subentry.path();
                                if subpath.is_file() {
                                    let name = subpath.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("");
                                    if name.contains("if02") || name.contains("-interface-02") {
                                        log!(logger, "Auto-detected candidate: {:?}", subpath);
                                        devices.push(subpath.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    } else if path.is_file() {
                        let name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");
                        if name.contains("USB") || name.contains("serial") || name.contains("ACM") {
                            log!(logger, "Auto-detected candidate (by-path): {:?}", path);
                            devices.push(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        let fallback_devices = [
            "/dev/ttyUSB0",
            "/dev/ttyUSB1",
            "/dev/ttyACM0",
            "/dev/ttyACM1",
            "/dev/ttyS0",
        ];
        for device in fallback_devices {
            if std::path::Path::new(device).exists() && !devices.contains(&device.to_string()) {
                log!(logger, "Auto-detected candidate (fallback): {}", device);
                devices.push(device.to_string());
            }
        }

        devices
    }
}