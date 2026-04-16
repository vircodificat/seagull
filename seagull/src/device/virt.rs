use std::sync::Mutex;
use std::time::Duration;

use crate::{Key, Stroke};
use crate::device::Device;

pub struct VirtualDevice {
    rx: std::sync::mpsc::Receiver<Stroke>,
}

impl VirtualDevice {
    pub fn new() -> VirtualDevice {
        let (tx, rx) = std::sync::mpsc::channel::<Stroke>();
        let dev = VirtualDevice {
            rx,
        };
        let _ = std::thread::spawn(|| run(tx));
        dev
    }
}

fn run(tx: std::sync::mpsc::Sender<Stroke>) {
    std::thread::sleep(std::time::Duration::from_millis(600));
    tx.send(Stroke::try_from_string("EU").unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(600));
    tx.send(Stroke::try_from_string("HROF").unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(600));
    tx.send(Stroke::try_from_string("U").unwrap()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(1000));
    tx.send(Stroke::try_from_string("U").unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(700));
    tx.send(Stroke::try_from_string("R").unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));
    tx.send(Stroke::try_from_string("HROF").unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5000));
    tx.send(Stroke::try_from_string("HREU").unwrap()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(1000));
    tx.send(Stroke::try_from_string("SKA").unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1000));
    tx.send(Stroke::try_from_string("TA").unwrap()).unwrap();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(u64::MAX));
    }
}

impl Device for VirtualDevice {
    fn read_stroke(&mut self) -> Stroke {
        self.rx.recv().unwrap()
    }
}
