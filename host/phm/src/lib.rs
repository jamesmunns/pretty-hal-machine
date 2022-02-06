use phm_icd::{ToMcu, ToMcuI2c, ToPc, ToPcI2c};
use postcard::{to_stdvec_cobs, CobsAccumulator, FeedResult};
use serialport::SerialPort;
use std::{
    io::{ErrorKind, self},
    time::{Duration, Instant},
};

/// The Pretty HAL Machine
///
/// This wraps a serial port connection to an embedded machine,
/// and implements various [embedded-hal](embedded-hal) traits.
pub struct Machine {
    port: Box<dyn SerialPort>,
    cobs_buf: CobsAccumulator<512>,
    command_timeout: Duration,
}

/// The main Error type
#[derive(Debug)]
pub enum Error {
    PhmSerial(io::Error),
    Postcard(postcard::Error),
    Timeout(Duration),

    // TODO: This probably needs some more context/nuance...
    ResponseError,
    Unknown,
}

impl From<postcard::Error> for Error {
    fn from(err: postcard::Error) -> Self {
        Error::Postcard(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::PhmSerial(err)
    }
}

impl Machine {
    pub fn from_port(port: Box<dyn SerialPort>) -> Result<Self, Error> {
        // TODO: some kind of sanity checking? Check version, protocol,
        // signs of life, anything?
        Ok(Self {
            port,
            cobs_buf: CobsAccumulator::new(),
            command_timeout: Duration::from_secs(3),
        })
    }

    /// Set the timeout for a full command to complete.
    ///
    /// This is not a single message timeout, but rather the timeout
    /// for a whole command (e.g. an I2C write) to execute. This is currently
    /// only checked/set host side, so endless loops on the embedded side are
    /// still possible.
    pub fn set_command_timeout(&mut self, timeout: Duration) {
        self.command_timeout = timeout;
    }

    fn poll(&mut self) -> Result<Vec<ToPc>, Error> {
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
                                responses.push(data);
                            } else {
                                return Err(Error::ResponseError);
                            }

                            remaining
                        }
                    };
                }
            }
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::TimedOut => {}
            Err(e) => {
                return Err(Error::PhmSerial(e));
            }
        }
        Ok(responses)
    }
}

impl embedded_hal::blocking::i2c::Write for Machine {
    type Error = Error;

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Error> {
        let mut bytes_hvec: heapless::Vec<u8, 64> = heapless::Vec::new();

        bytes.iter().for_each(|b| {
            let _ = bytes_hvec.push(*b).unwrap();
        });
        let msg = ToMcu::I2c(ToMcuI2c::Write {
            addr: address,
            output: bytes_hvec,
        });
        let ser_msg = to_stdvec_cobs(&msg)?;
        self.port.write_all(&ser_msg)?;

        let start = Instant::now();

        while start.elapsed() < self.command_timeout {
            for msg in self.poll()? {
                if let ToPc::I2c(ToPcI2c::WriteComplete { .. }) = msg {
                    return Ok(());
                }
            }

            // TODO: We should probably just use the `timeout` value of the serial
            // port, (e.g. don't delay at all), but I guess this is fine for now.
            std::thread::sleep(Duration::from_millis(10));
        }

        Err(Error::Timeout(self.command_timeout))
    }
}
