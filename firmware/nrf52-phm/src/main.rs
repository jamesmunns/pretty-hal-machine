#![no_main]
#![no_std]

use nrf52_phm as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = nrf52840_hal::pac, dispatchers = [SWI0_EGU0])]
mod app {
    use cortex_m::singleton;
    use defmt::unwrap;
    use heapless::spsc::Queue;
    use nrf52840_hal::{
        clocks::{ExternalOscillator, Internal, LfOscStopped},
        gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts, Level},
        pac::{SPIM2, TIMER0, TWIM0},
        spim::{Frequency as SpimFreq, Pins as SpimPins, Spim, MODE_0},
        twim::{Frequency as TwimFreq, Pins as TwimPins, Twim},
        usbd::{UsbPeripheral, Usbd},
        Clocks,
    };
    use nrf52_phm::monotonic::{ExtU32, MonoTimer};
    use phm_icd::{ToMcu, ToPc};
    use phm_worker::{
        comms::{CommsLink, InterfaceComms, WorkerComms},
        Worker,
    };
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
        interface_comms: InterfaceComms<8>,
        worker: Worker<WorkerComms<8>, Twim<TWIM0>, Spim<SPIM2>>,
        usb_serial: SerialPort<'static, Usbd<UsbPeripheral<'static>>>,
        usb_dev: UsbDevice<'static, Usbd<UsbPeripheral<'static>>>,
    }

    #[init(local = [
        usb_bus: Option<UsbBusAllocator<Usbd<UsbPeripheral<'static>>>> = None,
        incoming: Queue<ToMcu, 8> = Queue::new(),
        outgoing: Queue<Result<ToPc, ()>, 8> = Queue::new(),
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let device = cx.device;

        // Setup clocks early in the process. We need this for USB later
        let clocks = Clocks::new(device.CLOCK);
        let clocks = clocks.enable_ext_hfosc();
        let clocks =
            unwrap!(singleton!(: Clocks<ExternalOscillator, Internal, LfOscStopped> = clocks));

        // Configure the monotonic timer, currently using TIMER0, a 32-bit, 1MHz timer
        let mono = Monotonic::new(device.TIMER0);

        // Create GPIO ports for pin-mapping
        let port0 = P0Parts::new(device.P0);
        let port1 = P1Parts::new(device.P1);

        // Set up Twim
        let i2c = Twim::new(
            device.TWIM0,
            TwimPins {
                scl: port1.p1_01.into_floating_input().degrade(),
                sda: port1.p1_02.into_floating_input().degrade(),
            },
            TwimFreq::K100,
        );

        // Set up Spim
        let sck = port0.p0_08.into_push_pull_output(Level::Low).degrade();
        let mosi = port0.p0_04.into_push_pull_output(Level::Low).degrade();
        let miso = port0.p0_06.into_floating_input().degrade();
        let spi = Spim::new(
            device.SPIM2,
            SpimPins {
                sck,
                miso: Some(miso),
                mosi: Some(mosi),
            },
            SpimFreq::M2,
            MODE_0,
            0,
        );

        // Set up USB Serial Port
        let usb_bus = cx.local.usb_bus;
        usb_bus.replace(Usbd::new(UsbPeripheral::new(device.USBD, clocks)));
        let usb_serial = SerialPort::new(usb_bus.as_ref().unwrap());
        let usb_dev = UsbDeviceBuilder::new(usb_bus.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("OVAR Labs")
            .product("PHM Worker")
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

        let worker = Worker {
            io: worker_comms,
            i2c,
            spi,
        };

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
