#![no_main]
#![no_std]

use cortex_m::singleton;
use nrf52840_hal::{
    Clocks,
    usbd::{UsbPeripheral, Usbd},
    clocks::{ExternalOscillator, Internal, LfOscStopped},
    twim::{Twim, Pins as TwimPins, Frequency},
    gpio::{p1::Parts as P1Parts},
    pac::TWIM0,
};
use nrf52_phm as _; // global logger + panicking-behavior + memory layout
use embedded_hal::blocking::i2c::Write;

use defmt::unwrap;
use phm_icd::{ToMcu, ToPc, ToPcI2c, ToMcuI2c};
use postcard::{CobsAccumulator, FeedResult, to_vec_cobs};
use usb_device::{class_prelude::UsbBusAllocator, device::{UsbDeviceBuilder, UsbVidPid, UsbDevice}};
use usbd_serial::{SerialPort, USB_CLASS_CDC};
use heapless::spsc::{Queue, Producer, Consumer};

#[rtic::app(
    device = nrf52840_hal::pac,
    peripherals = true,
    monotonic = groundhog_nrf52::GlobalRollingTimer,

)]
const APP: () = {
    struct Resources {
        inc_prod: Producer<'static, ToMcu, 8>,
        inc_cons: Consumer<'static, ToMcu, 8>,
        out_prod: Producer<'static, Result<ToPc, ()>, 8>,
        out_cons: Consumer<'static, Result<ToPc, ()>, 8>,
        usb_serial: SerialPort<'static, Usbd<UsbPeripheral<'static>>>,
        cobs_buf: CobsAccumulator<512>,
        usb_dev: UsbDevice<'static, Usbd<UsbPeripheral<'static>>>,
        i2c: Twim<TWIM0>,
    }

    #[init(schedule = [usb_tick])]
    fn init(cx: init::Context) -> init::LateResources {
        let device = cx.device;

        // Ensure UICR NFC pins are disabled. This is to enable use of P0.09
        // and P0.10 which are by default mapped to NFC functionality.
        // disable_nfc_pins(&periphs);

        // Setup clocks early in the process. We need this for USB later
        let clocks = Clocks::new(device.CLOCK);
        let clocks = clocks.enable_ext_hfosc();
        let clocks = unwrap!(singleton!(: Clocks<ExternalOscillator, Internal, LfOscStopped> = clocks));

        // Configure the global timer, currently using TIMER0, a 32-bit, 1MHz timer
        groundhog_nrf52::GlobalRollingTimer::init(device.TIMER0);

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

        // ------------------------------------------------------------
        // Set up USB Serial Port
        //
        // NOTE: Use of singleton!() is acceptable as we can only call
        // this function safely once (as it consumes Peripherals).
        // ------------------------------------------------------------
        let (usb_dev, usb_serial) = {
            let usb_bus = Usbd::new(UsbPeripheral::new(device.USBD, clocks));
            let usb_bus = unwrap!(singleton!(:UsbBusAllocator<Usbd<UsbPeripheral>> = usb_bus));

            let usb_serial = SerialPort::new(usb_bus);
            let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("OVAR Labs")
                .product("Powerbus Mini")
                // TODO: Use some kind of unique ID. This will probably require another singleton,
                // as the storage must be static. Probably heapless::String -> singleton!()
                .serial_number("ajm123")
                .device_class(USB_CLASS_CDC)
                .max_packet_size_0(64) // (makes control transfers 8x faster)
                .build();

            (usb_dev, usb_serial)
        };

        let cobs_buf = CobsAccumulator::new();

        let incoming = unwrap!(singleton!(: Queue<ToMcu, 8> = Queue::new()));
        let outgoing = unwrap!(singleton!(: Queue<Result<ToPc, ()>, 8> = Queue::new()));

        let (inc_prod, inc_cons) = incoming.split();
        let (out_prod, out_cons) = outgoing.split();

        cx.schedule.usb_tick(cx.start).ok();

        init::LateResources {
            inc_prod,
            inc_cons,
            out_prod,
            out_cons,
            usb_serial,
            cobs_buf,
            usb_dev,
            i2c,
        }
    }

    #[task(schedule = [usb_tick], resources = [usb_serial, cobs_buf, inc_prod, out_cons, usb_dev])]
    fn usb_tick(cx: usb_tick::Context) {


        let usb_serial = cx.resources.usb_serial;
        let usb_dev = cx.resources.usb_dev;
        let cobs_buf = cx.resources.cobs_buf;
        let inc_prod = cx.resources.inc_prod;
        let out_cons = cx.resources.out_cons;

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
            },
            Ok(_) | Err(usb_device::UsbError::WouldBlock) => {},
            Err(_e) => defmt::panic!("Usb Error!"),
        }


        // Note: tick is in microseconds
        cx.schedule.usb_tick(cx.scheduled + 1_000i32).ok();
    }

    #[idle(resources = [inc_cons, out_prod, i2c])]
    fn idle(cx: idle::Context) -> ! {
        defmt::println!("Hello, world!");
        let i2c = cx.resources.i2c;

        loop {
            if let Some(data) = cx.resources.inc_cons.dequeue() {
                match data {
                    ToMcu::I2c(ToMcuI2c::Write { addr, output }) => {
                        // embedded_hal::blocking::i2c::Write
                        let msg = match Write::write(i2c, addr, &output) {
                            Ok(_) => {
                                Ok(ToPc::I2c(ToPcI2c::WriteComplete {
                                    addr: addr,
                                }))
                            }
                            Err(_) => {
                                Err(())
                            }
                        };

                        cx.resources.out_prod.enqueue(msg).ok();
                    },
                    ToMcu::I2c(msg) => {
                        defmt::println!("unhandled I2C! {:?}", msg);
                    }
                    ToMcu::Ping => {
                        let msg: Result<_, ()> = Ok(ToPc::Pong);
                        cx.resources.out_prod.enqueue(msg).ok();
                    }
                }
            }
        }
    }

    // Sacrificial hardware interrupts
    extern "C" {
        fn SWI0_EGU0();
    }
};


