use core::{marker::PhantomData, ops::Deref};

use stm32f4xx_hal::i2c::Pins;
use stm32f4xx_hal::pac::{I2C2, RCC};

/// I2C bus events
#[derive(Debug, PartialEq, Eq)]
pub enum I2CEvent {
    /// Start condition has been detected.
    Start,
    /// Restart condition has been detected.
    Restart,
    /// The controller requests data.
    TransferRead,
    /// The controller sends data.
    TransferWrite,
    /// Stop condition detected.
    Stop,
}

#[derive(Debug, Clone, Copy)]
enum State {
    Idle,
    Active(Direction),
    Read,
    Write,
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Tx,
    Rx,
}

pub struct I2CP<PINS: Pins<I2C2>> {
    i2c: I2C2,
    pins: PINS,
}

/// Provides Async features to I2C peripheral.
pub struct I2CPeripheralEventIterator<PINS: Pins<I2C2>> {
    i2c: I2CP<PINS>,
    state: State,
}

impl<PINS: Pins<I2C2>> I2CP<PINS> {
    /// Configures the I2C peripheral to work in peripheral mode
    ///
    /// The bus *MUST* be idle when this method is called.
    #[allow(clippy::type_complexity)]
    pub fn new_peripheral_event_iterator(
        i2c: I2C2,
        mut pins: PINS,
        addr: u16,
    ) -> I2CPeripheralEventIterator<PINS>
    where
        PINS: Pins<I2C2>,
    {
        let rcc = unsafe { &(*RCC::ptr()) };
        rcc.apb1enr.modify(|_, w| w.i2c2en().set_bit());
        rcc.apb1rstr.modify(|_, w| w.i2c2rst().set_bit());
        rcc.apb1rstr.modify(|_, w| w.i2c2rst().clear_bit());

        pins.set_alt_mode();

        i2c.cr1.modify(|_, w| w.pe().clear_bit());

        //i2c_reserved_addr(addr)
        i2c.oar1.write(|w| unsafe { w.add().bits(addr) });

        // Enable I2C block
        i2c.cr1.modify(|_, w| w.pe().set_bit());

        I2CPeripheralEventIterator {
            i2c: Self { i2c, pins },
            state: State::Idle,
        }
    }
}

impl<PINS: Pins<I2C2>> I2CPeripheralEventIterator<PINS> {
    /// Push up to `usize::min(TX_FIFO_SIZE, buf.len())` bytes to the TX FIFO.
    /// Returns the number of bytes pushed to the FIFO. Note this does *not* reflect how many bytes
    /// are effectively received by the controller.
    pub fn write(&mut self, buf: &[u8]) -> usize {
        let mut sent = 0;
        for &b in buf.iter() {
            while !self.i2c.i2c.sr1.read().tx_e().bit_is_set() {}
            self.i2c.i2c.dr.write(|w| unsafe { w.dr().bits(b) });
            while !self.i2c.i2c.sr1.read().btf().bit_is_set() {}
            sent += 1;
        }
        sent
    }

    fn addr_clear(&mut self) {
        let _ = self.i2c.i2c.sr1.read();
        let _ = self.i2c.i2c.sr2.read();
    }

    /// Pull up to `usize::min(RX_FIFO_SIZE, buf.len())` bytes from the RX FIFO.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let mut read = 0;

        for b in buf.iter_mut() {
            if !self.i2c.i2c.sr1.read().rx_ne().bit_is_set() {
                break;
            }

            *b = self.i2c.i2c.dr.read().dr().bits();
            read += 1;
        }
        read
    }
}
impl<PINS: Pins<I2C2>> Iterator for I2CPeripheralEventIterator<PINS> {
    type Item = I2CEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let stat = self.i2c.i2c.sr1.read();

        match self.state {
            State::Idle if stat.addr().bit_is_set() => {
                self.addr_clear();
                let dir = match self.i2c.i2c.sr2.read().tra().bit_is_set() {
                    true => Direction::Tx,
                    false => Direction::Rx,
                };
                self.state = State::Active(dir);
                Some(I2CEvent::Start)
            }
            State::Active(Direction::Rx) if stat.rx_ne().bit_is_set() => {
                self.state = State::Write;
                Some(I2CEvent::TransferWrite)
            }
            State::Active(Direction::Tx) => {
                // Clearing `rd_req` is used by the hardware to detect when the I2C block can stop
                // stretching the clock and start process the data pushed to the FIFO (if any).
                // This is done in `Self::write`.
                self.state = State::Read;
                Some(I2CEvent::TransferRead)
            }
            // State::Read if stat.rd_req().bit_is_set() => Some(I2CEvent::TransferRead),
            // State::Read if stat.restart_det().bit_is_set() => {
            //     self.i2c.i2c.ic_clr_restart_det.read();
            //     self.state = State::Active;
            //     Some(I2CEvent::Restart)
            // }
            // State::Write if !self.i2c.rx_fifo_empty() => Some(I2CEvent::TransferWrite),
            // State::Write if stat.restart_det().bit_is_set() => {
            //     self.i2c.i2c.ic_clr_restart_det.read();
            //     self.state = State::Active;
            //     Some(I2CEvent::Restart)
            // }
            _ if stat.stopf().bit_is_set() => {
                let _ = self.i2c.i2c.sr2.read();
                self.i2c.i2c.cr1.modify(|_, w| w);

                self.state = State::Idle;
                Some(I2CEvent::Stop)
            }
            _ if stat.af().bit_is_set() => {
                self.i2c.i2c.sr1.modify(|_, w| w.af().clear_bit());
                self.state = State::Idle;
                Some(I2CEvent::Stop)
            }
            _ => None,
        }
    }
}

// impl<PINS> I2CPeripheralEventIterator
// where
//     PINS: Pins<I2C2>,
// {
//     /// Releases the I2C peripheral and associated pins
//     #[allow(clippy::type_complexity)]
//     pub fn free(
//         self,
//         resets: &mut RESETS,
//     ) -> (Block, (Pin<Sda, FunctionI2C>, Pin<Scl, FunctionI2C>)) {
//         self.i2c.free(resets)
//     }
// }
