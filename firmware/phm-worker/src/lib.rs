#![no_std]

use embedded_hal::blocking::i2c;
use phm_icd::{Error as IcdError, ToMcu, ToMcuI2c, ToPc, ToPcI2c};

pub enum Error {
    Io,
    I2c,
    Internal,
}

pub mod comms {
    use heapless::spsc::{Consumer, Producer, Queue};
    use phm_icd::{Error as IcdError, ToMcu, ToPc};

    pub struct CommsLink<const N: usize> {
        pub to_pc: &'static mut Queue<Result<ToPc, IcdError>, N>,
        pub to_mcu: &'static mut Queue<ToMcu, N>,
    }

    impl<const N: usize> CommsLink<N> {
        pub fn split(self) -> (WorkerComms<N>, InterfaceComms<N>) {
            let (to_pc_prod, to_pc_cons) = self.to_pc.split();
            let (to_mcu_prod, to_mcu_cons) = self.to_mcu.split();

            (
                WorkerComms {
                    to_pc: to_pc_prod,
                    to_mcu: to_mcu_cons,
                },
                InterfaceComms {
                    to_pc: to_pc_cons,
                    to_mcu: to_mcu_prod,
                },
            )
        }
    }

    pub struct WorkerComms<const N: usize> {
        pub to_pc: Producer<'static, Result<ToPc, IcdError>, N>,
        pub to_mcu: Consumer<'static, ToMcu, N>,
    }

    impl<const N: usize> crate::WorkerIo for WorkerComms<N> {
        type Error = ();

        fn send(&mut self, msg: Result<ToPc, IcdError>) -> Result<(), Self::Error> {
            self.to_pc.enqueue(msg).map_err(drop)
        }

        fn receive(&mut self) -> Option<ToMcu> {
            self.to_mcu.dequeue()
        }
    }

    pub struct InterfaceComms<const N: usize> {
        pub to_pc: Consumer<'static, Result<ToPc, IcdError>, N>,
        pub to_mcu: Producer<'static, ToMcu, N>,
    }
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
    pub io: IO,
    pub i2c: I2C,
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
