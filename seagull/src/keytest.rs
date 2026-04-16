use seagull::read_stroke;

pub fn run(mut port: Box<dyn serialport::SerialPort>) {
    loop {
        let stroke = read_stroke(&mut *port);
        println!("{}", stroke.extended());
    }
}
