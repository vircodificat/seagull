#[cfg(test)]
mod tests;

use clap::Parser;
use seagull::device::{serial::SerialDevice, virt::VirtualDevice};

mod keytest;
mod game;

//const DEVICE: &str = "/dev/serial/by-id/usb-Wootpatoot_Lets_Split_v2-if02";
const DEVICE: &str = "/dev/serial/by-id/usb-StenoKeyboards_Polyglot-if02";


#[derive(Parser)]
struct Args {
    /// Run in key-test mode: print each stroke to stdout
    #[arg(short, long)]
    test: bool,

    /// Load sentences from FILENAME (one sentence per line)
    #[arg(long, value_name = "FILENAME", short, name="s")]
    sentence: Option<String>,
}

fn main() {
    let args = Args::parse();

    let device = Box::new(SerialDevice::new(DEVICE)
        .expect(&format!("Failed to open {DEVICE}")));

//    let device = Box::new(VirtualDevice::new());

    if args.test {
        keytest::run(device);
    } else {
        game::run(device, args.sentence.as_deref());
    }
}
