use nrf52840_hal::{
    pac::UARTE0,
    uarte::{UarteRx, UarteTx},
};

pub struct PhmUart {
    pub rx: UarteRx<UARTE0>,
    pub tx: UarteTx<UARTE0>,
}

impl embedded_hal::serial::Read<u8> for PhmUart {
    type Error = ();

    fn read(&mut self) -> Result<u8, nb::Error<Self::Error>> {
        embedded_hal::serial::Read::<u8>::read(&mut self.rx).map_err(|_| nb::Error::WouldBlock)
    }
}

impl embedded_hal::serial::Write<u8> for PhmUart {
    type Error = ();

    fn write(&mut self, output: u8) -> Result<(), nb::Error<Self::Error>> {
        embedded_hal::serial::Write::<u8>::write(&mut self.tx, output)
            .map_err(|_| nb::Error::WouldBlock)
    }

    fn flush(&mut self) -> Result<(), nb::Error<Self::Error>> {
        embedded_hal::serial::Write::<u8>::flush(&mut self.tx).map_err(|_| nb::Error::WouldBlock)
    }
}
