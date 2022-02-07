#![no_main]
#![no_std]

use blackpill_phm as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [USART1])]
mod app {
    use blackpill_phm::monotonic::{ExtU32, MonoTimer};
    use defmt::unwrap;
    use heapless::spsc::Queue;
    use phm_icd::{ToMcu, ToPc};
    use phm_worker::{
        comms::{CommsLink, InterfaceComms, WorkerComms},
        Worker,
    };
    use postcard::{to_vec_cobs, CobsAccumulator, FeedResult};
    use stm32f4xx_hal::{
        gpio::{
            gpioa::{PA5, PA6, PA7},
            gpiob::{PB8, PB9},
            Alternate, OpenDrain, PushPull,
        },
        i2c::I2c,
        otg_fs::{UsbBus, UsbBusType, USB},
        pac::{I2C1, SPI1},
        prelude::*,
        spi::{Mode, Phase, Polarity, Spi, TransferModeNormal},
    };
    use usb_device::{
        class_prelude::UsbBusAllocator,
        device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
    };
    use usbd_serial::{SerialPort, USB_CLASS_CDC};
    type BlackpillI2c = I2c<I2C1, (PB8<Alternate<OpenDrain, 4>>, PB9<Alternate<OpenDrain, 4>>)>;
    type BlackpillSpi = Spi<
        SPI1,
        (
            PA5<Alternate<PushPull, 5>>,
            PA6<Alternate<PushPull, 5>>,
            PA7<Alternate<PushPull, 5>>,
        ),
        TransferModeNormal,
    >;

    #[monotonic(binds = TIM2, default = true)]
    type Monotonic = MonoTimer<stm32f4xx_hal::pac::TIM2, 1_000_000>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        interface_comms: InterfaceComms<8>,
        worker: Worker<WorkerComms<8>, BlackpillI2c, BlackpillSpi>,
        usb_serial: SerialPort<'static, UsbBus<USB>>,
        usb_dev: UsbDevice<'static, UsbBus<USB>>,
    }

    #[init(local = [
        ep_memory: [u32; 1024] = [0; 1024],
        usb_bus: Option<UsbBusAllocator<UsbBusType>> = None,
        incoming: Queue<ToMcu, 8> = Queue::new(),
        outgoing: Queue<Result<ToPc, ()>, 8> = Queue::new(),
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let device = cx.device;

        // Set up the system clocks
        let rcc = device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.mhz()).require_pll48clk().freeze();

        // Configure the monotonic timer, currently using TIMER0, a 32-bit, 1MHz timer
        let mono = Monotonic::new(device.TIM2, &clocks);

        // Create GPIO ports for pin-mapping
        let gpioa = device.GPIOA.split();
        let gpiob = device.GPIOB.split();

        // Set up I2C
        let scl = gpiob.pb8.into_alternate_open_drain();
        let sda = gpiob.pb9.into_alternate_open_drain();
        let i2c = I2c::new(device.I2C1, (scl, sda), 400.khz(), &clocks);

        // Set up SPI
        let sck = gpioa.pa5.into_alternate();
        let miso = gpioa.pa6.into_alternate();
        let mosi = gpioa.pa7.into_alternate();
        let spi = Spi::new(
            device.SPI1,
            (sck, miso, mosi),
            Mode {
                polarity: Polarity::IdleLow,
                phase: Phase::CaptureOnFirstTransition,
            },
            2_000.khz(),
            &clocks,
        );

        // Set up USB
        let usb = USB {
            usb_global: device.OTG_FS_GLOBAL,
            usb_device: device.OTG_FS_DEVICE,
            usb_pwrclk: device.OTG_FS_PWRCLK,
            pin_dm: gpioa.pa11.into_alternate(),
            pin_dp: gpioa.pa12.into_alternate(),
            hclk: clocks.hclk(),
        };
        let usb_bus = cx.local.usb_bus;
        usb_bus.replace(UsbBus::new(usb, cx.local.ep_memory));

        // Set up USB Serial Port
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
