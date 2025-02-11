#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![cfg_attr(not(test), no_std)]

use core::cell::{Ref, RefCell, RefMut};
use core::fmt::Display;
use core::marker::PhantomData;
use core::ptr::NonNull;

use critical_section::{CriticalSection, Mutex};

/// Defines the errors at the crate level.
pub mod error {
    #[doc(hidden)]
    pub type Result<T, E = Error> = ::core::result::Result<T, E>;

    #[doc(hidden)]
    #[derive(Debug, PartialEq, Eq, thiserror::Error)]
    pub enum Error {
        #[error("undefined error")]
        Undefined,
    }
}

// Re-exports of errors.
pub use error::{Error, Result};

// Re-exports of macros.
pub use dedrv_macros::*;

/// The driver interface.
///
/// The driver does not include a state but only the implementation. Instead, the driver internal
/// state is stored by a [`Device`] instance.
pub trait Driver {
    /// The type of the internal driver state.
    type StateType: Send + Sized;

    /// The init function of the driver.
    ///
    /// This function initializes the driver internal state. It may include any side-effect that
    /// is required by the underlying hardware device to set up.
    fn init(state: &StateLock<Self>);

    /// The cleanup function of the driver.
    ///
    /// This function cleans up the driver internal state. This may include any side-effect that
    /// is required by the underlying hardware device to go back to a default state.
    fn cleanup(state: &StateLock<Self>);
}

/// Lock-protected driver internal state.
///
/// In concrete implementation, the driver internal state must be lock-protected to prevent from
/// race conditions (e.g. interrupt handler). The driver state being stored in a [`Device`], it
/// requires inner mutability. Both these constraints lead to use a [`RefCell`] inside a
/// `Mutex`. This offers the driver implementation to use the `critical-section` crate, which
/// implements a portable lock-based mechanism.
pub type StateLock<D> = Mutex<RefCell<<D as Driver>::StateType>>;

/// The device class tags.
///
/// Tags are used to statically restrain an [`Accessor`] to a unique device class.
pub mod tag {
    /// Defines a dummy (i.e. no-op) tag.
    pub struct NoTag;
}

/// A device instance.
///
/// Stores every device driver internal state and resources that are related to a given device
/// instance. This offers to share a driver implementation between many device instances but
/// specialize each
pub struct Device<D: Driver + 'static> {
    /// The lock-protected state for the driver that is related to this device instance.
    pub state: StateLock<D>,

    #[doc(hidden)]
    _drv: PhantomData<&'static D>,
}

impl<D: Driver> Device<D> {
    /// Create a new device instance.
    ///
    /// A device is `const`-constructible, so this function may be called from a top-level site
    /// (e.g. static global variable).
    ///
    /// At creation, the driver state of this device instance is zeroed.
    pub const fn new() -> Self {
        Device {
            state: Mutex::new(RefCell::new(unsafe { core::mem::zeroed() })),
            _drv: PhantomData,
        }
    }

    /// Call the [`Driver::init`] function of the driver on this device instance.
    #[inline(always)]
    pub fn init(&self) {
        D::init(&self.state)
    }

    /// Call the [`Driver::cleanup`] function of the driver on this device instance.
    #[inline(always)]
    pub fn cleanup(&self) {
        D::cleanup(&self.state)
    }

    /// Helper function to get access to the internal driver state from a critical section.
    #[inline(always)]
    pub fn state_ref<'d, 'cs>(&'d self, cs: CriticalSection<'cs>) -> Ref<'d, D::StateType>
    where
        'cs: 'd,
    {
        self.state.borrow_ref(cs)
    }

    /// Helper function to get access to the mutable internal driver state from a critical section.
    #[inline(always)]
    pub fn state_ref_mut<'d, 'cs>(&'d self, cs: CriticalSection<'cs>) -> RefMut<'d, D::StateType>
    where
        'cs: 'd,
    {
        self.state.borrow_ref_mut(cs)
    }

    /// Get a new accessor for the given class from this device.
    ///
    /// The type of an [`Accessor`] is tagged with a device class tag. This prevent from obtaining
    /// an accessor for a class that is not implemented by the underlying driver.
    pub fn accessor<Tag>(&self) -> Accessor<'_, D, Tag> {
        Accessor::new(self)
    }
}

impl<D: Driver> Default for Device<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Driver> Display for Device<D>
where
    D::StateType: Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        critical_section::with(|cs| write!(f, "{}", self.state_ref(cs)))
    }
}

impl<D: Driver> Drop for Device<D> {
    fn drop(&mut self) {}
}

/// An device class accessor.
pub struct Accessor<'d, D: Driver + 'static, Tag = tag::NoTag> {
    /// The owning device of this accessor.
    pub device: NonNull<Device<D>>,

    #[doc(hidden)]
    _marker: PhantomData<&'d Device<D>>,

    #[doc(hidden)]
    _tag: PhantomData<Tag>,
}

impl<'d, D: Driver, Tag> Accessor<'d, D, Tag> {
    /// Create a new accessor from an owning [`Device`].
    pub fn new(device: &'d Device<D>) -> Self {
        let device = unsafe { NonNull::new_unchecked(device as *const _ as *mut _) };
        Accessor {
            device,
            _marker: PhantomData,
            _tag: PhantomData,
        }
    }

    /// Helper function to get access to the inner device.
    #[inline(always)]
    pub fn inner(&self) -> &'d Device<D> {
        // SAFETY: The pointer is valid because of the lifetime of the accessor, which is at least
        // as long as the inner device.
        unsafe { self.device.as_ref() }
    }

    /// Helper function to get access to the internal driver state from a critical section.
    #[inline(always)]
    pub fn inner_state_ref<'a, 'cs>(&'a self, cs: CriticalSection<'cs>) -> Ref<'a, D::StateType>
    where
        'cs: 'a,
    {
        self.inner().state_ref(cs)
    }

    /// Helper function to get access to the mutable internal driver state from a critical section.
    #[inline(always)]
    pub fn inner_state_ref_mut<'a, 'cs>(
        &'a self,
        cs: CriticalSection<'cs>,
    ) -> RefMut<'a, D::StateType>
    where
        'cs: 'a,
    {
        self.inner().state_ref_mut(cs)
    }
}

/// Device descriptor to be stored in the `.dedrv.device.*` sections inside the linker script.
#[repr(C)]
pub struct Descriptor {
    path: &'static str,
    init: fn(*const ()),
    udata: *const (),
}

impl Descriptor {
    /// Create a new device descriptor.
    ///
    /// The `path` is a unique and short string identifier for the device. It provides a key to
    /// look up on the device in the static table (i.e. linker section).
    pub const fn new<D: Driver>(
        path: &'static str,
        device: &'static Device<D>,
        init: fn(*const ()),
    ) -> Self {
        Descriptor {
            path,
            init,
            udata: &raw const *device as *const _,
        }
    }
}

unsafe impl Sync for Descriptor {}

unsafe extern "C" {
    static __DEDRV_MARKER_DEVICE_START: usize;
    static __DEDRV_MARKER_DEVICE_END: usize;
}

/// Initialize all device drivers that are declared using the [`device`] attribute.
pub fn init() {
    let mut cursor = &raw const __DEDRV_MARKER_DEVICE_START as *const Descriptor;
    let end = &raw const __DEDRV_MARKER_DEVICE_END as *const Descriptor;

    while cursor < end {
        // SAFETY: At this point we guarantee that the cursor actually points to a `Descriptor`.
        // So, dereferencing the cursor is valid.
        let desc: &'static Descriptor = unsafe { &*cursor };
        (desc.init)(desc.udata);

        cursor = cursor.wrapping_add(1);
    }
}
