#![no_std]

use serde::{Serialize, Deserialize};
use heapless::Vec;


#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToMcu {
    I2c(ToMcuI2c),
    Ping,
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToMcuI2c {
    Write {
        output: Vec<u8, 64>,
    },
    Read {
        to_read: u32,
    },
    WriteThenRead {
        output: Vec<u8, 64>,
        to_read: u32,
    }
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToPc {
    I2c(ToPcI2c),
    Pong,
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToPcI2c {
    Read {
        data_read: Vec<u8, 64>,
    },
    WriteThenRead {
        data_read: Vec<u8, 64>,
    }
}
