use nrf52840_hal::{pac::UARTE0, uarte::Uarte};

pub struct PhmUart(pub Uarte<UARTE0>);

impl embedded_hal::blocking::serial::Write<u8> for PhmUart {
    type Error = ();

    fn bwrite_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.write(bytes).map_err(drop)?;
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
        let mut buf = [0_u8];
        match self.0.read(&mut buf) {
            Ok(_) => Ok(buf[0]),
            _ => Err(nb::Error::WouldBlock),
        }
    }
}
