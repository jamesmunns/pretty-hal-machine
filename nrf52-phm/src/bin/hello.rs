#![no_main]
#![no_std]

use nrf52_phm as _; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, world!");

    nrf52_phm::exit()
}
