#![no_std]

use embedded_hal::blocking::i2c;
use phm_icd::{Error as IcdError, ToMcu, ToMcuI2c, ToPc, ToPcI2c};

pub enum Error {
    Io,
    I2c,
    Internal,
}

pub trait WorkerIo {
    type Error;

    fn send(&mut self, msg: Result<ToPc, IcdError>) -> Result<(), Self::Error>;
    fn receive(&mut self) -> Option<ToMcu>;
}

pub struct Worker<IO, I2C>
where
    IO: WorkerIo,
    I2C: i2c::Write,
{
    io: IO,
    i2c: I2C,
}

impl<IO, I2C> Worker<IO, I2C>
where
    IO: WorkerIo,
    I2C: i2c::Write,
{
    pub fn step(&mut self) -> Result<(), Error> {
        while let Some(data) = self.io.receive() {
            let resp = match data {
                ToMcu::I2c(i2c) => self.process_i2c(i2c),
                ToMcu::Ping => {
                    defmt::info!("Received Ping! Responding...");
                    Ok(ToPc::Pong)
                }
            };
            self.io.send(resp.map_err(drop)).map_err(|_| Error::Io)?;
        }
        Ok(())
    }

    fn process_i2c(&mut self, i2c_cmd: ToMcuI2c) -> Result<ToPc, Error> {
        match i2c_cmd {
            ToMcuI2c::Write { addr, output } => {
                // embedded_hal::blocking::i2c::Write
                let msg = match i2c::Write::write(&mut self.i2c, addr, &output) {
                    Ok(_) => Ok(ToPc::I2c(ToPcI2c::WriteComplete { addr: addr })),
                    Err(_) => Err(Error::I2c),
                };
                msg
            }
            msg => {
                defmt::error!("unhandled I2C! {:?}", msg);
                Err(Error::Internal)
            }
        }
    }
}
