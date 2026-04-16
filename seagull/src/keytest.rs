use seagull::device::Device;

pub fn run(mut device: Box<dyn Device>) {
    loop {
        let stroke = device.read_stroke();
        println!("{}", stroke.extended());
    }
}
