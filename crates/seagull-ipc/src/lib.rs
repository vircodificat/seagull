use serde::{Deserialize, Serialize};
use zvariant::Type;

/// Messages sent from `seagull-tray` to `seagull-ime` over the session D-Bus.
///
/// Wire format is whatever `zvariant` produces for a Rust enum (a tagged
/// `(uv)` discriminant + variant payload). New variants can be added freely;
/// the IME's handler matches exhaustively.
#[derive(Serialize, Deserialize, Type, Debug, Clone)]
pub enum ImeMessage {
    /// A stroke read from the steno device.
    Stroke { bits: u32, is_control: bool },
}
