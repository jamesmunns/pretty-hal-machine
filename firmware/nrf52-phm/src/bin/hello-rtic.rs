#![no_main]
#![no_std]

<<<<<<< HEAD
use nrf52_phm as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [SWI0_EGU0])]
mod app {
    use cortex_m::singleton;
    use defmt::unwrap;
    use embedded_hal::blocking::i2c::Write;
    use heapless::spsc::{Consumer, Producer, Queue};
    use nrf52840_hal::{
        clocks::{ExternalOscillator, Internal, LfOscStopped},
        gpio::p1::Parts as P1Parts,
        pac::{TIMER0, TWIM0},
        twim::{Frequency, Pins as TwimPins, Twim},
        usbd::{UsbPeripheral, Usbd},
        Clocks,
    };
    use nrf52_phm::monotonic::{ExtU32, MonoTimer};
    use phm_icd::{ToMcu, ToMcuI2c, ToPc, ToPcI2c};
    use postcard::{to_vec_cobs, CobsAccumulator, FeedResult};
    use usb_device::{
        class_prelude::UsbBusAllocator,
        device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
    };
    use usbd_serial::{SerialPort, USB_CLASS_CDC};

    #[monotonic(binds = TIMER0, default = true)]
    type Monotonic = MonoTimer<TIMER0>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
=======
use cortex_m::singleton;
use embedded_hal::blocking::i2c::Write;
use nrf52840_hal::{
    clocks::{ExternalOscillator, Internal, LfOscStopped},
    gpio::p1::Parts as P1Parts,
    pac::TWIM0,
    twim::{Frequency, Pins as TwimPins, Twim},
    usbd::{UsbPeripheral, Usbd},
    Clocks,
};
use nrf52_phm as _; // global logger + panicking-behavior + memory layout

use defmt::unwrap;
use heapless::spsc::{Consumer, Producer, Queue};
use phm_icd::{ToMcu, ToMcuI2c, ToPc, ToPcI2c};
use postcard::{to_vec_cobs, CobsAccumulator, FeedResult};
use usb_device::{
    class_prelude::UsbBusAllocator,
    device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

#[rtic::app(
    device = nrf52840_hal::pac,
    peripherals = true,
    monotonic = groundhog_nrf52::GlobalRollingTimer,

)]
const APP: () = {
    struct Resources {
>>>>>>> main
        inc_prod: Producer<'static, ToMcu, 8>,
        inc_cons: Consumer<'static, ToMcu, 8>,
        out_prod: Producer<'static, Result<ToPc, ()>, 8>,
        out_cons: Consumer<'static, Result<ToPc, ()>, 8>,
        usb_serial: SerialPort<'static, Usbd<UsbPeripheral<'static>>>,
        usb_dev: UsbDevice<'static, Usbd<UsbPeripheral<'static>>>,
        i2c: Twim<TWIM0>,
    }

    #[init(local = [
        usb_bus: Option<UsbBusAllocator<Usbd<UsbPeripheral<'static>>>> = None,
        incoming: Queue<ToMcu, 8> = Queue::new(),
        outgoing: Queue<Result<ToPc, ()>, 8> = Queue::new(),
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let device = cx.device;

        // Ensure UICR NFC pins are disabled. This is to enable use of P0.09
        // and P0.10 which are by default mapped to NFC functionality.
        // disable_nfc_pins(&periphs);

        // Setup clocks early in the process. We need this for USB later
        let clocks = Clocks::new(device.CLOCK);
        let clocks = clocks.enable_ext_hfosc();
        let clocks =
            unwrap!(singleton!(: Clocks<ExternalOscillator, Internal, LfOscStopped> = clocks));

        // Configure the monotonic timer, currently using TIMER0, a 32-bit, 1MHz timer
        let mono = Monotonic::new(device.TIMER0);

        // // Create both GPIO ports for pin-mapping
        // let port0 = P0Parts::new(periphs.P0);
        let port1 = P1Parts::new(device.P1);

        let i2c = Twim::new(
            device.TWIM0,
            TwimPins {
                scl: port1.p1_01.into_floating_input().degrade(),
                sda: port1.p1_02.into_floating_input().degrade(),
            },
            Frequency::K100,
        );

        // Set up USB Serial Port
        let usb_bus = cx.local.usb_bus;
        usb_bus.replace(Usbd::new(UsbPeripheral::new(device.USBD, clocks)));
        let usb_serial = SerialPort::new(usb_bus.as_ref().unwrap());
        let usb_dev = UsbDeviceBuilder::new(usb_bus.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("OVAR Labs")
            .product("Powerbus Mini")
            // TODO: Use some kind of unique ID. This will probably require another singleton,
            // as the storage must be static. Probably heapless::String -> singleton!()
            .serial_number("ajm123")
            .device_class(USB_CLASS_CDC)
            .max_packet_size_0(64) // (makes control transfers 8x faster)
            .build();

        let (inc_prod, inc_cons) = cx.local.incoming.split();
        let (out_prod, out_cons) = cx.local.outgoing.split();

        usb_tick::spawn().ok();
        (
            Shared {},
            Local {
                inc_prod,
                inc_cons,
                out_prod,
                out_cons,
                usb_serial,
                usb_dev,
                i2c,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [usb_serial, inc_prod, out_cons, usb_dev, cobs_buf: CobsAccumulator<512> = CobsAccumulator::new()])]
    fn usb_tick(cx: usb_tick::Context) {
<<<<<<< HEAD
        let usb_serial = cx.local.usb_serial;
        let usb_dev = cx.local.usb_dev;
        let cobs_buf = cx.local.cobs_buf;
        let inc_prod = cx.local.inc_prod;
        let out_cons = cx.local.out_cons;
=======
        let usb_serial = cx.resources.usb_serial;
        let usb_dev = cx.resources.usb_dev;
        let cobs_buf = cx.resources.cobs_buf;
        let inc_prod = cx.resources.inc_prod;
        let out_cons = cx.resources.out_cons;
>>>>>>> main

        let mut buf = [0u8; 128];

        usb_dev.poll(&mut [usb_serial]);

        if let Some(out) = out_cons.dequeue() {
            if let Ok(ser_msg) = to_vec_cobs::<_, 128>(&out) {
                usb_serial.write(&ser_msg).ok();
            } else {
                defmt::panic!("Serialization error!");
            }
        }

        match usb_serial.read(&mut buf) {
            Ok(sz) if sz > 0 => {
                let buf = &buf[..sz];
                let mut window = &buf[..];

                'cobs: while !window.is_empty() {
                    window = match cobs_buf.feed::<phm_icd::ToMcu>(&window) {
                        FeedResult::Consumed => break 'cobs,
                        FeedResult::OverFull(new_wind) => new_wind,
                        FeedResult::DeserError(new_wind) => new_wind,
                        FeedResult::Success { data, remaining } => {
                            defmt::println!("got: {:?}", data);
                            inc_prod.enqueue(data).ok();
                            remaining
                        }
                    };
                }
            }
            Ok(_) | Err(usb_device::UsbError::WouldBlock) => {}
            Err(_e) => defmt::panic!("Usb Error!"),
        }

<<<<<<< HEAD
        usb_tick::spawn_after(1.millis()).ok();
=======
        // Note: tick is in microseconds
        cx.schedule.usb_tick(cx.scheduled + 1_000i32).ok();
>>>>>>> main
    }

    #[idle(local = [inc_cons, out_prod, i2c])]
    fn idle(cx: idle::Context) -> ! {
        defmt::println!("Hello, world!");
        let i2c = cx.local.i2c;

        loop {
            if let Some(data) = cx.local.inc_cons.dequeue() {
                match data {
                    ToMcu::I2c(ToMcuI2c::Write { addr, output }) => {
                        // embedded_hal::blocking::i2c::Write
                        let msg = match Write::write(i2c, addr, &output) {
                            Ok(_) => Ok(ToPc::I2c(ToPcI2c::WriteComplete { addr: addr })),
                            Err(_) => Err(()),
                        };

<<<<<<< HEAD
                        cx.local.out_prod.enqueue(msg).ok();
=======
                        cx.resources.out_prod.enqueue(msg).ok();
>>>>>>> main
                    }
                    ToMcu::I2c(msg) => {
                        defmt::println!("unhandled I2C! {:?}", msg);
                    }
                    ToMcu::Ping => {
                        let msg: Result<_, ()> = Ok(ToPc::Pong);
                        cx.local.out_prod.enqueue(msg).ok();
                    }
                }
            }
        }
    }
<<<<<<< HEAD
}
=======

    // Sacrificial hardware interrupts
    extern "C" {
        fn SWI0_EGU0();
    }
};
>>>>>>> main
