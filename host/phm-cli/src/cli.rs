use std::num::ParseIntError;

use clap::{Args, Parser, Subcommand};
use embedded_hal::prelude::{
    _embedded_hal_blocking_i2c_Read, _embedded_hal_blocking_i2c_Write,
    _embedded_hal_blocking_i2c_WriteRead,
};
use phm::Machine;

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
    /// Write bytes to the given address
    #[clap(name = "write")]
    I2CWrite(I2CWrite),
    /// Read count bytes from the given address
    #[clap(name = "read")]
    I2CRead(I2CRead),
    /// Write-Read bytes to and from the given address
    #[clap(name = "write-read")]
    WriteRead(WriteRead),
}

#[derive(Args, Debug)]
struct I2CWrite {
    /// The address to write to.
    #[clap(short = 'a')]
    address: u8,
    /// Bytes to write to the address. Should be given as a comma-separated list of hex values. For example: "0xA0,0xAB,0x11".
    #[clap(short = 'b', long = "write")]
    write_bytes: String,
}

#[derive(Args, Debug)]
struct I2CRead {
    /// The address to write to.
    #[clap(short = 'a')]
    address: u8,
    /// Number of bytes to read.
    #[clap(long = "read-ct")]
    read_count: usize,
}

#[derive(Args, Debug)]
struct WriteRead {
    /// The address to write to.
    #[clap(short = 'a')]
    address: u8,
    #[clap(short = 'b', long = "bytes")]
    write_bytes: String,
    /// Bytes to write to the address. Should be given as a comma-separated list of hex values. For example: "0xA0,0xAB,0x11".
    #[clap(long = "read-ct")]
    read_count: usize,
}

impl PhmCli {
    pub fn run(&self, machine: &mut Machine) -> Result<(), phm::Error> {
        match self {
            PhmCli::I2C(cmd) => match &cmd.command {
                I2CCommand::I2CWrite(args) => {
                    let bytes =
                        parse_bytes(&args.write_bytes).map_err(|_| phm::Error::InvalidParameter)?;

                    machine.write(args.address, &bytes)
                }
                I2CCommand::I2CRead(args) => {
                    let mut buffer = vec![0u8; args.read_count];

                    machine.read(args.address, &mut buffer)
                }
                I2CCommand::WriteRead(args) => {
                    let bytes =
                        parse_bytes(&args.write_bytes).map_err(|_| phm::Error::InvalidParameter)?;
                    let mut buffer = vec![0u8; args.read_count];

                    machine.write_read(args.address, &bytes, &mut buffer)
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
