//! # Pretty HAL Machine Worker
//!
//! This crate contains the device-agnostic logic that is shared among
//! all implementations of the Pretty HAL Machine worker on different MCUs.

#![no_std]

use embedded_hal::blocking::i2c;
use phm_icd::{Error as IcdError, ToMcu, ToMcuI2c, ToPc, ToPcI2c};

/// The worker Error type
#[derive(Debug, defmt::Format, Eq, PartialEq)]
pub enum Error {
    Io,
    I2c,
    Internal,
}

/// Helper types for MCU-to-PC communications
pub mod comms {
    use heapless::spsc::{Consumer, Producer, Queue};
    use phm_icd::{Error as IcdError, ToMcu, ToPc};

    /// A wrapper structure for statically allocated bidirectional queues
    pub struct CommsLink<const N: usize> {
        pub to_pc: &'static mut Queue<Result<ToPc, IcdError>, N>,
        pub to_mcu: &'static mut Queue<ToMcu, N>,
    }

    impl<const N: usize> CommsLink<N> {
        /// Split the CommsLink into Worker and Interface halves.
        ///
        /// The WorkerComms half is intended to be used with a [Worker](crate::Worker) implmentation,
        /// The InterfaceComms half is intended to be used where bytes are send and received to the
        /// PC, such as the USB Serial handler function
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

    /// The Worker half of the the CommsLink type.
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

    /// Serial Interface half of the CommsLink type.
    pub struct InterfaceComms<const N: usize> {
        pub to_pc: Consumer<'static, Result<ToPc, IcdError>, N>,
        pub to_mcu: Producer<'static, ToMcu, N>,
    }
}

/// A trait for managing messages to or from a Worker
pub trait WorkerIo {
    type Error;

    /// Send a message FROM the worker, TO the PC.
    fn send(&mut self, msg: Result<ToPc, IcdError>) -> Result<(), Self::Error>;

    /// Receive a message FROM the PC, TO the worker
    fn receive(&mut self) -> Option<ToMcu>;
}

/// A Pretty HAL Machine Worker
///
/// This struct is intended to contain all of the shared logic between workers.
/// It is highly generic, which should allow the logic to execute regardless of
/// the MCU the worker is executing on.
pub struct Worker<IO, I2C>
where
    IO: WorkerIo,
    I2C: i2c::Write + i2c::Read + i2c::WriteRead,
{
    pub io: IO,
    pub i2c: I2C,
}

impl<IO, I2C> Worker<IO, I2C>
where
    IO: WorkerIo,
    I2C: i2c::Write + i2c::Read + i2c::WriteRead,
{
    /// Process any pending messages to the worker
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
                match i2c::Write::write(&mut self.i2c, addr, &output) {
                    Ok(_) => Ok(ToPc::I2c(ToPcI2c::WriteComplete { addr })),
                    Err(_) => Err(Error::I2c),
                }
            }
            ToMcuI2c::Read { addr, to_read } => {
                let mut buf = [0u8; 64];
                let to_read_usize = to_read as usize;

                if to_read_usize > buf.len() {
                    return Err(Error::I2c);
                }
                let buf_slice = &mut buf[..to_read_usize];

                match i2c::Read::read(&mut self.i2c, addr, buf_slice) {
                    Ok(_) => Ok(ToPc::I2c(ToPcI2c::Read {
                        addr,
                        data_read: buf_slice.iter().cloned().collect(),
                    })),
                    Err(_) => Err(Error::I2c),
                }
            }
            ToMcuI2c::WriteThenRead {
                addr,
                output,
                to_read,
            } => {
                let mut buf = [0u8; 64];
                let to_read_usize = to_read as usize;

                if to_read_usize > buf.len() {
                    return Err(Error::I2c);
                }
                let buf_slice = &mut buf[..to_read_usize];

                match i2c::WriteRead::write_read(&mut self.i2c, addr, &output, buf_slice) {
                    Ok(_) => Ok(ToPc::I2c(ToPcI2c::WriteThenRead {
                        addr,
                        data_read: buf_slice.iter().cloned().collect(),
                    })),
                    Err(_) => Err(Error::I2c),
                }
            }
        }
    }
}
