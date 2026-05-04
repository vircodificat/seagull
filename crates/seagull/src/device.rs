pub mod serial;
pub mod virt;

use crate::Stroke;

pub trait Device: Send {
    fn read_stroke(&mut self) -> Result<Keycode, std::io::Error>;
}

pub struct Keycode {
    stroke: Stroke,
    is_control: bool
}

impl Keycode {
    pub fn stroke(&self) -> Stroke {
        self.stroke
    }

    pub fn is_control(&self) -> bool {
        self.is_control
    }
}
