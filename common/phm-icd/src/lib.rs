#![no_std]

use heapless::Vec;
use serde::{Deserialize, Serialize};

// TODO: Something better than this
pub type Error = ();

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToMcu {
    I2c(ToMcuI2c),
    Spi(ToMcuSpi),
    Uart(ToMcuUart),
    Ping,
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToMcuI2c {
    Write {
        addr: u8,
        output: Vec<u8, 64>,
    },
    Read {
        addr: u8,
        to_read: u32,
    },
    WriteThenRead {
        addr: u8,
        output: Vec<u8, 64>,
        to_read: u32,
    },
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToMcuSpi {
    Write { output: Vec<u8, 64> },
    Transfer { output: Vec<u8, 64> },
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToMcuUart {
    Write { output: Vec<u8, 64> },
    Flush,
    Read,
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToPc {
    I2c(ToPcI2c),
    Spi(ToPcSpi),
    Uart(ToPcUart),
    Pong,
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToPcI2c {
    WriteComplete { addr: u8 },
    Read { addr: u8, data_read: Vec<u8, 64> },
    WriteThenRead { addr: u8, data_read: Vec<u8, 64> },
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToPcSpi {
    WriteComplete,
    Transfer { data_read: Vec<u8, 64> },
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToPcUart {
    WriteComplete,
    Read { data_read: Vec<u8, 64> },
}
