use std::time::Duration;
use clap::Parser;

mod keytest;
mod game;

const DEVICE: &str = "/dev/serial/by-id/usb-Wootpatoot_Lets_Split_v2-if02";

#[derive(Parser)]
struct Args {
    /// Run in key-test mode: print each stroke to stdout
    #[arg(short, long)]
    test: bool,
}

fn main() {
    let args = Args::parse();

    let port = serialport::new(DEVICE, 9600)
        .timeout(Duration::from_millis(10))
        .open()
        .expect(&format!("Failed to open {DEVICE}"));

    if args.test {
        keytest::run(port);
    } else {
        game::run(port);
    }
}
