use dedrv::{Accessor, Device, Driver};

/// Defines a peripheral class.
#[dedrv::class]
pub trait Gpio {
    fn get_value(&self) -> u32;
    fn set_value(&mut self, value: u32);
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;

    use dedrv::{Descriptor, StateLock};

    use super::*;

    // User implementaiton.
    struct GpioDriver;

    // User implementaiton.
    impl Driver for GpioDriver {
        type StateType = u32;

        // TODO: use device instead of state or resources.
        fn init(_state: &StateLock<Self>) {}
        fn cleanup(_state: &StateLock<Self>) {}
    }

    // User implementaiton.
    impl driver::Gpio for GpioDriver {
        fn get_value(state: &StateLock<Self>) -> u32 {
            critical_section::with(|cs| *state.borrow_ref(cs))
        }

        fn set_value(state: &StateLock<Self>, value: u32) {
            critical_section::with(|cs| {
                *state.borrow_ref_mut(cs) = value;
            })
        }
    }

    #[test]
    fn it_should_init_device() {
        static DEVICE: Device<GpioDriver> = Device::new();
        DEVICE.init();
    }

    #[test]
    fn it_should_not_compile_accessor_after_drop() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/units/accessor_after_drop.rs");
    }

    #[test]
    fn it_should_use_class_accessor_to_modify_state() {
        static DEVICE: Device<GpioDriver> = Device::new();
        DEVICE.init();

        let mut gpio = DEVICE.accessor::<tag::Gpio>();
        critical_section::with(|cs| assert_that!(*gpio.inner_state_ref(cs), eq(0)));

        gpio.set_value(32);
        critical_section::with(|cs| assert_that!(*gpio.inner_state_ref(cs), eq(32)));
    }

    #[test]
    fn it_should_populate_dedrv_linker_section() {
        static DEVICE: Device<GpioDriver> = Device::new();

        fn __dedrv_device_init(ptr: *const ()) {
            let device: &'static _ = unsafe { &*(ptr as *const Device<GpioDriver>) };
            device.init();
        }

        #[allow(unused)]
        #[link_section = ".dedrv.device.gpio0"]
        static DESCRIPTOR: Descriptor = Descriptor::new("/gpio0", &DEVICE, __dedrv_device_init);
    }
}
