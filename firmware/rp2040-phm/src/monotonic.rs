//! # [`Monotonic`] implementation based on RP2040's `Timer` peripheral.
//!
//! Uses [`fugit`] as underlying time library.
//!
//! [`fugit`]: https://docs.rs/crate/fugit
//! [`Monotonic`]: https://docs.rs/rtic-monotonic

pub use fugit::{self, ExtU64};
use rp_pico::pac::{RESETS, TIMER};
use rtic_monotonic::Monotonic;

/// RP2040 `Timer` implementation for `rtic_monotonic::Monotonic`.
pub struct Rp2040Monotonic {
    timer: TIMER,
}

impl Rp2040Monotonic {
    /// Create a new `Monotonic` based on RP2040's `Timer` peripheral.
    pub fn new(timer: TIMER) -> Self {
        Self { timer }
    }
}

impl Monotonic for Rp2040Monotonic {
    const DISABLE_INTERRUPT_ON_EMPTY_QUEUE: bool = false;

    type Instant = fugit::TimerInstantU64<1_000_000>;
    type Duration = fugit::TimerDurationU64<1_000_000>;

    fn now(&mut self) -> Self::Instant {
        let mut hi0 = self.timer.timerawh.read().bits();
        loop {
            let low = self.timer.timerawl.read().bits();
            let hi1 = self.timer.timerawh.read().bits();
            if hi0 == hi1 {
                break Self::Instant::from_ticks((u64::from(hi0) << 32) | u64::from(low));
            }
            hi0 = hi1;
        }
    }

    unsafe fn reset(&mut self) {
        let resets = &*RESETS::ptr();
        resets.reset.modify(|_, w| w.timer().clear_bit());
        while resets.reset_done.read().timer().bit_is_clear() {}
        self.timer.inte.modify(|_, w| w.alarm_0().set_bit());
    }

    fn set_compare(&mut self, instant: Self::Instant) {
        let now = self.now();

        let max = u32::MAX as u64;

        // Since the timer may or may not overflow based on the requested compare val, we check
        // how many ticks are left.
        let val = match instant.checked_duration_since(now) {
            Some(x) if x.ticks() <= max => instant.duration_since_epoch().ticks() & max, // Will not overflow
            _ => 0, // Will overflow or in the past, set the same value as after overflow to not get extra interrupts
        };

        self.timer.alarm0.write(|w| unsafe { w.bits(val as u32) });
    }

    fn clear_compare_flag(&mut self) {
        self.timer.intr.modify(|_, w| w.alarm_0().set_bit());
    }

    fn zero() -> Self::Instant {
        Self::Instant::from_ticks(0)
    }
}
