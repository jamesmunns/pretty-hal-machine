use embedded_hal::prelude::_embedded_hal_blocking_i2c_Write;
use phm_icd::{ToMcu, ToMcuI2c, ToPc, ToPcI2c};
use postcard::{to_stdvec_cobs, CobsAccumulator, FeedResult};
use serialport::SerialPort;
use std::{
    io::ErrorKind,
    time::{Duration, Instant},
};

fn main() -> Result<(), ()> {
    println!("Hello, world!");

    let mut dport = None;

    for port in serialport::available_ports().unwrap() {
        println!("{:?}", port);
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

    let mut ehal = EhalSerial {
        port,
        cobs_buf: CobsAccumulator::new(),
    };

    let mut last_send = Instant::now();

    loop {
        if last_send.elapsed() >= Duration::from_secs(1) {
            println!("Sending command!");
            ehal.write(0x42, &[1, 2, 3, 4]).unwrap();
            last_send = Instant::now();
        }
    }
}

struct EhalSerial {
    port: Box<dyn SerialPort>,
    cobs_buf: CobsAccumulator<512>,
}

impl EhalSerial {
    fn poll(&mut self) -> Vec<ToPc> {
        let mut responses = vec![];
        let mut buf = [0u8; 1024];

        // read from stdin and push it to the decoder
        match self.port.read(&mut buf) {
            Ok(n) if n > 0 => {
                let buf = &buf[..n];
                let mut window = &buf[..];

                'cobs: while !window.is_empty() {
                    window = match self.cobs_buf.feed::<Result<phm_icd::ToPc, ()>>(&window) {
                        FeedResult::Consumed => break 'cobs,
                        FeedResult::OverFull(new_wind) => new_wind,
                        FeedResult::DeserError(new_wind) => new_wind,
                        FeedResult::Success { data, remaining } => {
                            // Do something with `data: MyData` here.
                            if let Ok(data) = data {
                                println!("got: {:?}", data);
                                responses.push(data);
                            } else {
                                eprintln!("I2C failed!");
                            }

                            remaining
                        }
                    };
                }
            }
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::TimedOut => {}
            Err(e) => {
                panic!("ERR: {:?}", e);
            }
        }
        responses
    }
}

impl embedded_hal::blocking::i2c::Write for EhalSerial {
    type Error = ();

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut bytes_hvec: heapless::Vec<u8, 64> = heapless::Vec::new();

        bytes.iter().for_each(|b| {
            let _ = bytes_hvec.push(*b).unwrap();
        });
        let msg = ToMcu::I2c(ToMcuI2c::Write {
            addr: address,
            output: bytes_hvec,
        });
        let ser_msg = to_stdvec_cobs(&msg).map_err(drop)?;
        self.port.write_all(&ser_msg).map_err(drop)?;

        loop {
            for msg in self.poll() {
                if let ToPc::I2c(ToPcI2c::WriteComplete { .. }) = msg {
                    return Ok(());
                } else {
                    eprintln!("Unexpected msg (ignoring)! {:?}", msg);
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
