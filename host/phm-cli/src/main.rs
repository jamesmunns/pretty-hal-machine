use std::time::{Duration, Instant};
use phm::Machine;

fn main() -> Result<(), ()> {
    println!("Hello, world!");

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
        eprintln!();
        eprintln!("Error: Didn't find a `powerbus mini` device! Is the firmware running?");
        eprintln!();
        return Ok(());
    };

    let port = serialport::new(dport.port_name, 115200)
        .timeout(Duration::from_millis(5))
        .open()
        .map_err(drop)?;

    let mut ehal = Machine::from_port(port).unwrap();

    let mut last_send = Instant::now();

    loop {
        if last_send.elapsed() >= Duration::from_secs(1) {
            println!("Sending command!");
            embedded_hal::blocking::i2c::Write::write(&mut ehal, 0x42, &[1, 2, 3, 4]).unwrap();
            last_send = Instant::now();
        }
    }
}
