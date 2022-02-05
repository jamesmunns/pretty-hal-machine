#![no_std]

use serde::{Serialize, Deserialize};


#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
enum ToMcu<'a>{
    #[serde(borrow)]
    I2c(ToMcuI2c<'a>),
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
enum ToMcuI2c<'a> {
    Write {
        output: &'a [u8]
    },
    Read {
        to_read: u32,
    },
    WriteThenRead {
        output: &'a [u8],
        to_read: u32,
    }
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
enum ToPc<'a>{
    #[serde(borrow)]
    I2c(ToPcI2c<'a>),
}

#[cfg_attr(feature = "use-defmt", derive(defmt::Format))]
#[derive(Debug, Serialize, Deserialize)]
enum ToPcI2c<'a> {
    Read {
        data_read: &'a [u8],
    },
    WriteThenRead {
        data_read: &'a [u8],
    }
}
