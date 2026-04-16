use std::time::Duration;

use serialport::SerialPort;

use crate::{Key, Stroke};
use crate::device::{Device, Keycode};

pub struct SerialDevice(Box<dyn SerialPort>);

impl SerialDevice {
    pub fn new(device: &str) -> Result<SerialDevice, serialport::Error> {
        let port = serialport::new(device, 9600)
            .timeout(Duration::from_millis(10))
            .open()?;
        Ok(SerialDevice(port))
    }

    fn port(&mut self) -> &mut dyn SerialPort {
        self.0.as_mut()
    }
}

impl Device for SerialDevice {
    fn read_stroke(&mut self) -> Keycode {
        let mut buf = [0; 6];
        let mut total_amount = 0;

        loop {
            let buf_slice = &mut buf[total_amount..6];
            match self.port().read(buf_slice) {
                Ok(amount) => {
                    total_amount += amount;
                },
                Err(_e) => {
                }
            }

            if total_amount == 6 {
                break;
            }
        }

        let value: u64 =
            (buf[0] as u64) |
            (buf[1] as u64) << 8 |
            (buf[2] as u64) << 16 |
            (buf[3] as u64) << 24 |
            (buf[4] as u64) << 32 |
            (buf[5] as u64) << 40;

        let mut keys = vec![];
        for (key, key_value) in KEY_CODES {
            if value & key_value == *key_value {
                keys.push(*key);
            }
        }

        const LEFT_CONTROL_KEY:  u64 = 0x20;
        const RIGHT_CONTROL_KEY: u64 = 0x10;

        let is_control =
            (value & LEFT_CONTROL_KEY != 0)
            || (value & RIGHT_CONTROL_KEY != 0);

        let stroke = Stroke::new(keys.as_slice());
        Keycode {
            stroke,
            is_control,
        }
    }
}

const KEY_CODES: &[(Key, u64)] = &[
    (Key::LeftS, 0x000000004080), // S1
    (Key::LeftS, 0x000000002080), // S2
    (Key::LeftT, 0x000000001080),
    (Key::LeftK, 0x000000000880),
    (Key::LeftP, 0x000000000480),
    (Key::LeftW, 0x000000000280),
    (Key::LeftH, 0x000000000180),
    (Key::LeftR, 0x000000400080),

    (Key::MiddleA, 0x000000200080),
    (Key::MiddleO, 0x000000100080),
    (Key::MiddleStar, 0x000000080080),
    (Key::MiddleStar, 0x000020000080),
    (Key::MiddleStar, 0x000000040080),
    (Key::MiddleStar, 0x000010000080),
    (Key::MiddleE, 0x000008000080),
    (Key::MiddleU, 0x000004000080),

    (Key::RightF, 0x000002000080),
    (Key::RightR, 0x000001000080),
    (Key::RightP, 0x004000000080),
    (Key::RightB, 0x002000000080),
    (Key::RightL, 0x001000000080),
    (Key::RightG, 0x000800000080),
    (Key::RightT, 0x000400000080),
    (Key::RightS, 0x000200000080),
    (Key::RightD, 0x000100000080),
    (Key::RightZ, 0x010000000080),
];

