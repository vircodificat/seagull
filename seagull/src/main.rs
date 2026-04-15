use std::time::Duration;
use seagull::read_stroke;

const DEVICE: &'static str = "/dev/serial/by-id/usb-Wootpatoot_Lets_Split_v2-if02";

fn main() {
    let mut port = serialport::new(DEVICE, 9600)
        .timeout(Duration::from_millis(10))
        .open()
        .expect(&format!("Failed to open {DEVICE}"));

    loop {
        let stroke = read_stroke(&mut *port);
        println!("{}", stroke.extended());
    }
}
