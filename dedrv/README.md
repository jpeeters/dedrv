This library has two main goals. The first one is to decouple device drivers internals from
actual device instances, leading to drivers that are agnostic the hardware device (i.e.
peripheral) instance and configuration. This is achieved by storing the per-device [`Driver`] state
and configuration at the [`Device`] level, saying it is stateful. Then, the driver implementation
may be entirely stateless and generic amongst all instances of the same hardware device.

The second goal is to offer a convenient device driver management at the time of boot and
initialization. This is achieved by putting device instances inside a dedicated linker
section, which can be iterated at runtime. Moreover, this linker section acts as a device
registry and could be looked up for a specific device with a unique identifier. This is the
first step of a minimal and efficient device tree storage for application to use.
