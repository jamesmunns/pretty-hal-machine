#![no_main]
#![no_std]

use defmt::Format;
use nrf52_phm as _; // global logger + panicking-behavior + memory layout // <- derive attribute

#[derive(Format)]
struct S1<T> {
    x: u8,
    y: T,
}

#[derive(Format)]
struct S2 {
    z: u8,
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let s = S1 {
        x: 42,
        y: S2 { z: 43 },
    };
    defmt::println!("s={:?}", s);
    let x = 42;
    defmt::println!("x={=u8}", x);

    nrf52_phm::exit()
}
