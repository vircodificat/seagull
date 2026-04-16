use seagull::device::Device;

pub fn run(mut device: Box<dyn Device>) {
    loop {
        let keycode = device.read_stroke();
        println!("{}{}", if keycode.is_control() { "+" } else { "" }, keycode.stroke().extended());
    }
}
