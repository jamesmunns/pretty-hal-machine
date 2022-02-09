use rp_pico::{
    hal::{uart::Enabled as UartEnabled, uart::UartPeripheral as Uarte},
    pac::UART0,
};

pub struct PhmUart(pub Uarte<UartEnabled, UART0>);

impl embedded_hal::blocking::serial::Write<u8> for PhmUart {
    type Error = ();

    fn bwrite_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.write_full_blocking(bytes);
        Ok(())
    }

    fn bflush(&mut self) -> Result<(), Self::Error> {
        // TODO
        Ok(())
    }
}

impl embedded_hal::serial::Read<u8> for PhmUart {
    type Error = ();

    fn read(&mut self) -> Result<u8, nb::Error<Self::Error>> {
        match self.0.read() {
            Ok(b) => Ok(b),
            _ => Err(nb::Error::WouldBlock),
        }
    }
}
