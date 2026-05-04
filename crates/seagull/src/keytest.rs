use seagull::device::Device;

pub fn run(mut device: Box<dyn Device>) {
    loop {
        let keycode = device.read_stroke().expect("read_stroke failed");
        println!("{}{}", if keycode.is_control() { "+" } else { "" }, keycode.stroke().extended());
    }
}
