use log::{info, warn};
use seagull::Stroke;
use seagull_ipc::ImeMessage;
use tokio::sync::mpsc;
use zbus::interface;

/// D-Bus object served at `/at/vircodific/seagull/IME` under the bus name
/// `at.vircodific.seagull.IME` (interface `at.vircodific.seagull.IME.Control`). Receives
/// `ImeMessage` variants from `seagull-tray` and forwards them to the
/// main loop via an `mpsc` channel.
pub struct Control {
    tx: mpsc::Sender<(Stroke, bool)>,
}

impl Control {
    pub fn new(tx: mpsc::Sender<(Stroke, bool)>) -> Self {
        Self { tx }
    }
}

#[interface(name = "at.vircodific.seagull.IME.Control")]
impl Control {
    async fn send(&self, msg: ImeMessage) -> zbus::fdo::Result<()> {
        match msg {
            ImeMessage::Stroke { bits, is_control } => {
                let stroke = Stroke::from_bits(bits);
                info!("Control.Send: stroke={stroke} (control={is_control})");
                if self.tx.send((stroke, is_control)).await.is_err() {
                    warn!("Control.Send: stroke channel closed");
                    return Err(zbus::fdo::Error::Failed(
                        "IME stroke channel closed".into(),
                    ));
                }
                Ok(())
            }
        }
    }
}
