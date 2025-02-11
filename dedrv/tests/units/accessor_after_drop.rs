#![no_std]

use dedrv::Device;

fn main() {}
        let device: Device<GpioDriver> = Device::new();
        device.init();

        let mut accessor = device.accessor::<tag::Gpio>();

        drop(device);

        accessor.set_value(32);
}
