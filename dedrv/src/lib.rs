#![no_std]

use core::cell::{Ref, RefCell, RefMut};
use core::fmt::Display;
use core::marker::PhantomData;
use core::ptr::NonNull;

use critical_section::{CriticalSection, Mutex};

/// Defines the errors that can be returned.
pub mod error {
    pub type Result<T, E = Error> = ::core::result::Result<T, E>;

    #[derive(Debug, PartialEq, Eq, thiserror::Error)]
    pub enum Error {
        #[error("undefined error")]
        Undefined,
    }
}

/// Re-exports.
pub use error::{Error, Result};

/// Re-exports macros.
pub use dedrv_macros::*;

/// Defines the driver interface.
pub trait Driver {
    /// The type of the internal driver state.
    /// Note: it must be `Sized` because it must be statically constructible.
    type StateType: Send + Sized;

    /// The init function of the driver.
    /// Note: multiple calls to the `init` function must be idempotent.
    fn init(state: &StateLock<Self>);

    /// The cleanup function of the driver.
    /// Note: multiple calls to the `cleanup` function must be idempotent.
    fn cleanup(state: &StateLock<Self>);
}

/// Lock to protect the `RefCell` of the driver state. This offers the driver implementation to use
/// `critical-section` crate for protecting access to the driver internal state.
pub type StateLock<D> = Mutex<RefCell<<D as Driver>::StateType>>;

/// The tag module, including all tags possble for an accessor.
pub mod tag {
    /// Defines an accessor tag that is not associated to any peripheral class.
    pub struct NoTag;
}

/// Defines a device from the hardware point of view.
///
/// It includes the driver state so that a same driver can be used for multiple instances of the
/// same hardware device (e.g. gpio, adc, timers...).
pub struct Device<D: Driver + 'static> {
    /// The driver state owned by the device.
    pub state: StateLock<D>,

    // The driver implementation phantom type. Because the driver is stateless, there is no need
    // for keeping an instance of it.
    _drv: PhantomData<&'static D>,
}

impl<D: Driver> Device<D> {
    /// Create a new device with the internal driver state set to zereos.
    pub const fn new() -> Self {
        Device {
            state: Mutex::new(RefCell::new(unsafe { core::mem::zeroed() })),
            _drv: PhantomData,
        }
    }

    /// Call the `init` function of the driver. This call is idempotent until the next call to
    /// the `cleanup` function.
    pub fn init(&self) {
        D::init(&self.state)
    }

    /// Call the `cleanup` function of the driver. This call is idempotent until the next call to
    /// the `init` function.
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

/// Defines an accessor.
pub struct Accessor<'d, D: Driver + 'static, Tag = tag::NoTag> {
    pub device: NonNull<Device<D>>,
    _marker: PhantomData<&'d Device<D>>,
    _tag: PhantomData<Tag>,
}

impl<'d, D: Driver, Tag> Accessor<'d, D, Tag> {
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

/// Device descriptor to be put inside linker section.
#[repr(C)]
pub struct Descriptor {
    pub path: &'static str,
    pub init: fn(*const ()),
    pub udata: *const (),
}

impl Descriptor {
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

/// Initialize device drivers.
pub fn init() {
    let mut cursor = &raw const __DEDRV_MARKER_DEVICE_START as *const Descriptor;
    let end = &raw const __DEDRV_MARKER_DEVICE_END as *const Descriptor;

    while cursor < end {
        // Call driver init function on its internal state stored in the device instance.
        let desc: &'static Descriptor = unsafe { &*cursor };
        (desc.init)(desc.udata);

        // Go to next descriptor.
        cursor = cursor.wrapping_add(1);
    }
}
