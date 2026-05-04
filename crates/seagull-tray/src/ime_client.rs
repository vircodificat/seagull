use log::{info, warn};
use seagull_ipc::ImeMessage;
use zbus::Connection;
use zbus::proxy;

#[proxy(
    interface = "at.vircodific.seagull.IME.Control",
    default_service = "at.vircodific.seagull.IME",
    default_path = "/at/vircodific/seagull/IME"
)]
pub trait Control {
    async fn send(&self, msg: ImeMessage) -> zbus::Result<()>;
}

/// Wraps a zbus Control proxy and tracks whether the IME owns its bus name
/// so we can drop strokes silently when it isn't running.
pub struct ImeClient {
    proxy: ControlProxy<'static>,
    available: std::sync::atomic::AtomicBool,
}

impl ImeClient {
    pub async fn new(connection: &Connection) -> zbus::Result<Self> {
        let proxy = ControlProxy::new(connection).await?;
        let available = match check_owner(connection).await {
            Ok(v) => v,
            Err(e) => {
                warn!("NameHasOwner check failed at startup: {e}");
                false
            }
        };
        if available {
            info!("IME present on bus at startup");
        } else {
            info!("IME not present on bus at startup");
        }
        Ok(Self {
            proxy,
            available: std::sync::atomic::AtomicBool::new(available),
        })
    }

    pub fn set_available(&self, v: bool) {
        self.available
            .store(v, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_available(&self) -> bool {
        self.available.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn send(&self, msg: ImeMessage) {
        if !self.is_available() {
            info!("Dropping stroke: IME not on bus");
            return;
        }
        if let Err(e) = self.proxy.send(msg).await {
            warn!("Send to IME failed: {e}");
            self.set_available(false);
        }
    }
}

async fn check_owner(connection: &Connection) -> zbus::Result<bool> {
    let dbus = zbus::fdo::DBusProxy::new(connection).await?;
    let name: zbus::names::BusName<'static> = "at.vircodific.seagull.IME".try_into()?;
    Ok(dbus.name_has_owner(name).await?)
}

/// Spawns a background task that watches the D-Bus `NameOwnerChanged` signal for
/// `at.vircodific.seagull.IME`. When the signal fires (IME appears/disappears on the bus),
/// updates the client's availability flag so strokes are only sent when the IME is running.
/// Signal-driven approach is more efficient and responsive than polling.
pub fn spawn_name_watcher(connection: Connection, client: std::sync::Arc<ImeClient>) {
    tokio::spawn(async move {
        let dbus = match zbus::fdo::DBusProxy::new(&connection).await {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to create DBusProxy for name watching: {e}");
                return;
            }
        };
        let mut stream = match dbus.receive_name_owner_changed().await {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to subscribe to NameOwnerChanged: {e}");
                return;
            }
        };
        use futures_util::StreamExt;
        while let Some(signal) = stream.next().await {
            let args = match signal.args() {
                Ok(a) => a,
                Err(e) => {
                    warn!("Bad NameOwnerChanged args: {e}");
                    continue;
                }
            };
            if args.name.as_str() != "at.vircodific.seagull.IME" {
                continue;
            }
            let now_owned = !args.new_owner.is_none();
            info!(
                "IME bus name {} (was: {}, now: {})",
                if now_owned { "appeared" } else { "disappeared" },
                args.old_owner
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("<none>"),
                args.new_owner
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("<none>"),
            );
            client.set_available(now_owned);
        }
    });
}
