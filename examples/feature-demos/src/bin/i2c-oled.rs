// $ cargo run --bin i2c-oled
use core::fmt::Write;
use phm::Machine;
use std::time::{Duration, Instant};

use ssd1306::{
    prelude::*, rotation::DisplayRotation, size::DisplaySize128x64, I2CDisplayInterface, Ssd1306,
};

fn main() -> Result<(), ()> {
    println!("I2C OLED display driver demo!");

    let mut dport = None;

    for port in serialport::available_ports().unwrap() {
        if let serialport::SerialPortType::UsbPort(serialport::UsbPortInfo {
            serial_number: Some(sn),
            ..
        }) = &port.port_type
        {
            if sn.as_str() == "ajm123" {
                dport = Some(port.clone());
                break;
            }
        }
    }

    let dport = if let Some(port) = dport {
        port
    } else {
        eprintln!("Error: No `Pretty hal machine` connected!");
        return Ok(());
    };

    let port = serialport::new(dport.port_name, 115200)
        .timeout(Duration::from_millis(5))
        .open()
        .map_err(drop)?;

    let ehal = Machine::from_port(port).unwrap();

    // Configure the OLED display.
    let interface = I2CDisplayInterface::new(ehal);
    let mut disp =
        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0).into_terminal_mode();
    disp.init().ok();
    disp.clear().ok();
    disp.write_str("Hello world!\n").ok();

    let mut last_send = Instant::now();

    loop {
        if last_send.elapsed() >= Duration::from_secs(1) {
            println!("Sending command!");
            disp.write_str("PHM!\n").ok();
            last_send = Instant::now();
        }
    }
}
