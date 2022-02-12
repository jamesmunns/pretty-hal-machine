use clap::Parser;
use phm::Machine;
use std::time::Duration;

use crate::cli::PhmCli;

mod cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = PhmCli::parse();

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
        .map_err(|_| "Error: failed to create port")?;

    let mut ehal = Machine::from_port(port).unwrap();

    match cmd.run(&mut ehal) {
        Ok(out) => {
            println!("{out}");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
