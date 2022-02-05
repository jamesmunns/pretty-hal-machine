use std::{time::{Duration, Instant}, io::ErrorKind};

fn main() -> Result<(), ()> {
    println!("Hello, world!");

    let mut dport = None;

    for port in serialport::available_ports().unwrap() {
        if let serialport::SerialPortType::UsbPort(serialport::UsbPortInfo { serial_number: Some(sn), .. }) = &port.port_type {
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

    let mut port = serialport::new(dport.port_name, 115200)
        .timeout(Duration::from_millis(5))
        .open()
        .map_err(drop)?;

    let mut last_send = Instant::now();

    let mut buf = [0u8; 1024];

    loop {
        if last_send.elapsed() >= Duration::from_secs(1) {
            port.write_all(b"hello, usb!").map_err(drop)?;
            last_send = Instant::now();
        }

        // read from stdin and push it to the decoder
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                println!("Got: {}", std::str::from_utf8(&buf[..n]).map_err(drop)?);
            }
            Ok(_) => {},
            Err(e) if e.kind() == ErrorKind::TimedOut => {},
            Err(e) => {
                println!("ERR: {:?}", e);
                return Err(());
            }
        }
    }


}
