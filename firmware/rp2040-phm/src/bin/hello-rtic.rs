#![no_main]
#![no_std]

use rp2040_phm as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = rp_pico::hal::pac, dispatchers = [XIP_IRQ])]
mod app {
    use embedded_hal::blocking::i2c::Write;
    use embedded_time::rate::Extensions;
    use heapless::spsc::{Consumer, Producer, Queue};
    use phm_icd::{ToMcu, ToMcuI2c, ToPc, ToPcI2c};
    use postcard::{to_vec_cobs, CobsAccumulator, FeedResult};
    use rp2040_monotonic::*;
    use rp_pico::{
        hal::{
            clocks::init_clocks_and_plls,
            gpio::pin::{
                bank0::{Gpio16, Gpio17},
                FunctionI2C, Pin,
            },
            usb::UsbBus,
            watchdog::Watchdog,
            Sio, I2C,
        },
        pac::I2C0,
        XOSC_CRYSTAL_FREQ,
    };
    use usb_device::{class_prelude::*, prelude::*};
    use usbd_serial::{SerialPort, USB_CLASS_CDC};

    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type Monotonic = Rp2040Monotonic;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        inc_prod: Producer<'static, ToMcu, 8>,
        inc_cons: Consumer<'static, ToMcu, 8>,
        out_prod: Producer<'static, Result<ToPc, ()>, 8>,
        out_cons: Consumer<'static, Result<ToPc, ()>, 8>,
        usb_dev: UsbDevice<'static, UsbBus>,
        usb_serial: SerialPort<'static, UsbBus>,
        i2c: I2C<I2C0, (Pin<Gpio16, FunctionI2C>, Pin<Gpio17, FunctionI2C>)>,
    }

    #[init(local = [
        usb_bus: Option<UsbBusAllocator<UsbBus>> = None,
        incoming: Queue<ToMcu, 8> = Queue::new(),
        outgoing: Queue<Result<ToPc, ()>, 8> = Queue::new(),
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let device = cx.device;

        // Setup clocks
        let mut resets = device.RESETS;
        let mut watchdog = Watchdog::new(device.WATCHDOG);
        let clocks = init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            device.XOSC,
            device.CLOCKS,
            device.PLL_SYS,
            device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        // Configure the monotonic timer
        let mono = Monotonic::new(device.TIMER);

        // The single-cycle I/O block controls our GPIO pins
        let sio = Sio::new(device.SIO);

        // Set the pins up according to their function on this particular board
        let pins = rp_pico::Pins::new(
            device.IO_BANK0,
            device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        // Configure I2C pins
        let sda_pin = pins.gpio16.into_mode::<rp_pico::hal::gpio::FunctionI2C>();
        let scl_pin = pins.gpio17.into_mode::<rp_pico::hal::gpio::FunctionI2C>();

        // Set up the I2C driver
        let i2c = I2C::i2c0(
            device.I2C0,
            sda_pin,
            scl_pin,
            100.kHz(),
            &mut resets,
            clocks.peripheral_clock,
        );

        // Set up USB Serial Port
        let usb_bus = cx.local.usb_bus;
        usb_bus.replace(UsbBusAllocator::new(UsbBus::new(
            device.USBCTRL_REGS,
            device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut resets,
        )));
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

    #[task(binds=USBCTRL_IRQ, local = [usb_serial, inc_prod, out_cons, usb_dev, cobs_buf: CobsAccumulator<512> = CobsAccumulator::new()])]
    fn on_usb(cx: on_usb::Context) {
        let usb_serial = cx.local.usb_serial;
        let usb_dev = cx.local.usb_dev;
        let cobs_buf = cx.local.cobs_buf;
        let inc_prod = cx.local.inc_prod;
        let out_cons = cx.local.out_cons;

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

                        cx.local.out_prod.enqueue(msg).ok();
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
}
