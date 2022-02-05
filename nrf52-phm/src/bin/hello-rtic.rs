#![no_main]
#![no_std]

use nrf52_phm as _; // global logger + panicking-behavior + memory layout

    // monotonic = groundhog_nrf52::GlobalRollingTimer


#[rtic::app(
    device = nrf52840_hal::pac,
    peripherals = true,
)]
const APP: () = {
    struct Resources {
        lol: bool,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        init::LateResources {
            lol: true,
        }
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::println!("Hello, world!");

        loop {
            cortex_m::asm::nop();
        }
    }
};


