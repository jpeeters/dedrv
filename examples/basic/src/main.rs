#![no_std]
#![no_main]

use defmt::{debug, info};

use defmt_rtt as _;
use panic_probe as _;

use dedrv::{Accessor, Device, Driver};

#[derive(Debug)]
#[non_exhaustive]
pub enum PinMode {
    Input,
    Output,
}

#[dedrv::class]
pub trait Gpio {
    fn configure(&self, pin: u8, mode: PinMode);
}

struct GpioDriver;

impl Driver for GpioDriver {
    type StateType = ();

    fn init(_: &dedrv::StateLock<Self>) {
        info!("init gpio driver");
    }

    fn cleanup(_: &dedrv::StateLock<Self>) {}
}

impl driver::Gpio for GpioDriver {
    fn configure(_: &dedrv::StateLock<Self>, pin: u8, _: PinMode) {
        debug!("configure gpio pin {}", pin);
    }
}

#[dedrv::device(path = "/gpio0")]
static GPIO0: Device<GpioDriver> = Device::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    info!("Hello, World from Rust!");

    // Init drivers.
    dedrv::init();

    let gpio = GPIO0.accessor::<tag::Gpio>();
    gpio.configure(0 /* pin */, PinMode::Output);

    info!("init ok");

    #[allow(clippy::empty_loop)]
    loop {}
}
