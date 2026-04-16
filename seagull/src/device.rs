pub mod serial;

use crate::Stroke;

pub trait Device: Send {
    fn read_stroke(&mut self) -> Stroke;
}
