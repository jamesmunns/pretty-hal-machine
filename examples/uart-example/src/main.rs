use phm::Machine;
use std::time::{Duration, Instant};

fn main() -> Result<(), ()> {
    println!("Uart example");

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

    embedded_hal::blocking::serial::Write::<u8>::bflush(&mut ehal).unwrap();

    let mut last_send = Instant::now();

    loop {
        if last_send.elapsed() >= Duration::from_secs(1) {
            let str = "Pretty HAL machine!\n";
            print!("TX: {}", str);
            embedded_hal::blocking::serial::Write::<u8>::bwrite_all(&mut ehal, str.as_bytes())
                .unwrap();

            last_send = Instant::now();
        }
        while let Ok(b) = embedded_hal::serial::Read::<u8>::read(&mut ehal) {
            println!("RX: {:x}", b);
        }
    }
}
