#![no_main]
#![no_std]

use rp2040_phm as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = rp_pico::hal::pac, dispatchers = [XIP_IRQ])]
mod app {
    use defmt::unwrap;
    use embedded_time::rate::Extensions;
    use heapless::spsc::Queue;
    use phm_icd::{ToMcu, ToPc};
    use phm_worker::{
        comms::{CommsLink, InterfaceComms, WorkerComms},
        Worker,
    };
    use postcard::{to_vec_cobs, CobsAccumulator, FeedResult};
    use rp2040_monotonic::*;
    use rp_pico::{
        hal::{
            clocks::init_clocks_and_plls,
            gpio::pin::{
                bank0::{Gpio16, Gpio17},
                FunctionI2C, FunctionSpi, FunctionUart, Pin,
            },
            spi::{self, Spi},
            uart::{common_configs as UartConfig, Enabled as UartEnabled, UartPeripheral},
            usb::UsbBus,
            watchdog::Watchdog,
            Clock, Sio, I2C,
        },
        pac::{I2C0, SPI0, UART0},
        XOSC_CRYSTAL_FREQ,
    };
    use usb_device::{class_prelude::*, prelude::*};
    use usbd_serial::{SerialPort, USB_CLASS_CDC};
    type PhmI2c = I2C<I2C0, (Pin<Gpio16, FunctionI2C>, Pin<Gpio17, FunctionI2C>)>;
    type PhmSpi = Spi<spi::Enabled, SPI0, 8>;
    type PhmUart = UartPeripheral<UartEnabled, UART0>;

    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type Monotonic = Rp2040Monotonic;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        interface_comms: InterfaceComms<8>,
        worker: Worker<WorkerComms<8>, PhmI2c, PhmSpi, PhmUart>,
        usb_serial: SerialPort<'static, UsbBus>,
        usb_dev: UsbDevice<'static, UsbBus>,
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
        let pclk_freq = clocks.peripheral_clock.freq();

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

        // Set up the I2C pins and driver
        let sda_pin = pins.gpio16.into_mode::<FunctionI2C>();
        let scl_pin = pins.gpio17.into_mode::<FunctionI2C>();
        let i2c = I2C::i2c0(
            device.I2C0,
            sda_pin,
            scl_pin,
            100.kHz(),
            &mut resets,
            clocks.peripheral_clock,
        );

        // Set up the SPI pins and driver
        let _sck = pins.gpio2.into_mode::<FunctionSpi>();
        let _mosi = pins.gpio3.into_mode::<FunctionSpi>();
        let _miso = pins.gpio4.into_mode::<FunctionSpi>();
        let spi = Spi::<_, _, 8>::new(device.SPI0).init(
            &mut resets,
            pclk_freq,
            2_000_000_u32.Hz(),
            &embedded_hal::spi::MODE_0,
        );

        // Set up UART
        let _tx_pin = pins.gpio0.into_mode::<FunctionUart>();
        let _rx_pin = pins.gpio1.into_mode::<FunctionUart>();
        let uart = UartPeripheral::new(device.UART0, &mut resets)
            .enable(UartConfig::_9600_8_N_1, pclk_freq)
            .unwrap();

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

        let comms = CommsLink {
            to_pc: cx.local.outgoing,
            to_mcu: cx.local.incoming,
        };

        let (worker_comms, interface_comms) = comms.split();

        let worker = Worker::new(worker_comms, i2c, spi, uart);

        usb_tick::spawn().ok();
        (
            Shared {},
            Local {
                worker,
                interface_comms,
                usb_serial,
                usb_dev,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [usb_serial, interface_comms, usb_dev, cobs_buf: CobsAccumulator<512> = CobsAccumulator::new()])]
    fn usb_tick(cx: usb_tick::Context) {
        let usb_serial = cx.local.usb_serial;
        let usb_dev = cx.local.usb_dev;
        let cobs_buf = cx.local.cobs_buf;
        let interface_comms = cx.local.interface_comms;

        let mut buf = [0u8; 128];

        usb_dev.poll(&mut [usb_serial]);

        if let Some(out) = interface_comms.to_pc.dequeue() {
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
                            interface_comms.to_mcu.enqueue(data).ok();
                            remaining
                        }
                    };
                }
            }
            Ok(_) | Err(usb_device::UsbError::WouldBlock) => {}
            Err(_e) => defmt::panic!("Usb Error!"),
        }
        usb_tick::spawn_after(1.millis()).ok();
    }

    #[idle(local = [worker])]
    fn idle(cx: idle::Context) -> ! {
        defmt::println!("Hello, world!");
        let worker = cx.local.worker;

        loop {
            unwrap!(worker.step().map_err(drop));
        }
    }
}
