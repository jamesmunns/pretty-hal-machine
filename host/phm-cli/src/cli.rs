use std::num::ParseIntError;

use clap::{Args, Parser, Subcommand};
use embedded_hal::prelude::{_embedded_hal_blocking_i2c_Read, _embedded_hal_blocking_i2c_Write};
use phm::Machine;
use serialport::SerialPortInfo;

#[derive(Parser, Debug)]
pub enum PhmCli {
    I2C(I2C),
}

#[derive(Parser, Debug)]
pub struct I2C {
    #[clap(subcommand)]
    command: I2CCommand,
}

#[derive(Subcommand, Debug)]
enum I2CCommand {
    #[clap(name = "write")]
    I2CWrite(I2CWrite),
    #[clap(name = "read")]
    I2CRead(I2CRead),
    #[clap(name = "read-write")]
    I2CReadWrite(I2CReadWrite),
}

#[derive(Args, Debug)]
struct I2CWrite {
    #[clap(short = 'a')]
    address: u8,
    #[clap(short = 'b', long = "write")]
    write_bytes: String,
}

#[derive(Args, Debug)]
struct I2CRead {
    #[clap(short = 'a')]
    address: u8,
    #[clap(long = "read-ct")]
    read_count: usize,
}

#[derive(Args, Debug)]
struct I2CReadWrite {
    #[clap(short = 'a')]
    address: u8,
    #[clap(short = 'b', long = "bytes")]
    write_bytes: String,
    #[clap(long = "read-ct")]
    read_count: usize,
}

impl PhmCli {
    pub fn run(machine: &mut Machine) -> Result<(), phm::Error> {
        let cmd = PhmCli::parse();

        match cmd {
            PhmCli::I2C(cmd) => match cmd.command {
                I2CCommand::I2CWrite(args) => {
                    let bytes =
                        parse_bytes(&args.write_bytes).map_err(|_| phm::Error::InvalidParameter)?;
                    machine.write(args.address, &bytes)
                }
                I2CCommand::I2CRead(args) => {
                    let mut buffer = vec![0u8; args.read_count];

                    machine.read(args.address, buffer.as_mut())
                }
                I2CCommand::I2CReadWrite(args) => {
                    let bytes =
                        parse_bytes(&args.write_bytes).map_err(|_| phm::Error::InvalidParameter)?;
                    machine.write(args.address, &bytes)?;

                    let mut buffer = vec![0u8; args.read_count];
                    machine.read(args.address, buffer.as_mut())
                }
            },
        }
    }
}

fn parse_bytes(input: &str) -> Result<Vec<u8>, ParseIntError> {
    let mut bytes: Vec<u8> = Vec::new();
    for b in input.split(',') {
        let without_prefix = b.trim_start_matches("0x");
        let byte = u8::from_str_radix(without_prefix, 16)?;
        bytes.push(byte);
    }

    Ok(bytes)
}
