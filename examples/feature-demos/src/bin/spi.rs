// $ cargo run --bin spi
use phm::Machine;
use std::time::{Duration, Instant};

fn main() -> Result<(), ()> {
    println!("SPI transfer demo!");

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

    let mut ehal = Machine::from_port(port).unwrap();

    let mut last_send = Instant::now();

    loop {
        if last_send.elapsed() >= Duration::from_secs(1) {
            // println!("Sending I2C command!");
            // embedded_hal::blocking::i2c::Write::write(&mut ehal, 0x42, &[1, 2, 3, 4]).unwrap();

            let mut buf = [1, 2, 3, 4];
            println!("Sending SPI: {:?}", buf);
            embedded_hal::blocking::spi::Transfer::transfer(&mut ehal, &mut buf).unwrap();
            println!("Received SPI: {:?}", buf);
            last_send = Instant::now();
        }
    }
}