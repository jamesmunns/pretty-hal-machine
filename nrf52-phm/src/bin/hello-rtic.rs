#![no_main]
#![no_std]

use cortex_m::singleton;
use nrf52840_hal::{Clocks, pac::Peripherals, usbd::{UsbPeripheral, Usbd}, clocks::{ExternalOscillator, Internal, LfOscStopped}};
use nrf52_phm as _; // global logger + panicking-behavior + memory layout

    // monotonic = groundhog_nrf52::GlobalRollingTimer
use defmt::unwrap;
use usb_device::{class_prelude::UsbBusAllocator, device::{UsbDeviceBuilder, UsbVidPid}};
use usbd_serial::{SerialPort, USB_CLASS_CDC};
use groundhog_nrf52::GlobalRollingTimer;
use groundhog::RollingTimer;

#[rtic::app(
    device = nrf52840_hal::pac,
    peripherals = true,
)]
const APP: () = {
    struct Resources {
        device: Option<nrf52840_hal::pac::Peripherals>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        init::LateResources {
            device: Some(cx.device),
        }
    }

    #[idle(resources = [device])]
    fn idle(cx: idle::Context) -> ! {
        defmt::println!("Hello, world!");

        let device = cx.resources.device.take().unwrap();

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
        // let port1 = P1Parts::new(periphs.P1);

        // ------------------------------------------------------------
        // Set up USB Serial Port
        //
        // NOTE: Use of singleton!() is acceptable as we can only call
        // this function safely once (as it consumes Peripherals).
        //
        // TODO: Use this to bridge the anachro network
        // ------------------------------------------------------------
        let (mut usb_dev, mut usb_serial) = {
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

        let mut buf = [0u8; 128];



        // usb_serial.write(msg.deref())




        let timer = GlobalRollingTimer::default();

        loop {
            let start = timer.get_ticks();
            while timer.micros_since(start) <= 1000 {
                usb_dev.poll(&mut [&mut usb_serial]);

                loop {
                    match usb_serial.read(&mut buf) {
                        Ok(sz) if sz > 0 => {
                            defmt::println!("Got {=usize} bytes!", sz);
                        },
                        Ok(_) | Err(usb_device::UsbError::WouldBlock) => break,
                        Err(_e) => defmt::panic!("Usb Error!"),
                    }
                }
            }
        }
    }
};


