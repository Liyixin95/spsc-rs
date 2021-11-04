use crate::loom::{AtomicU8, Ordering};
use core::ptr;
use std::cell::UnsafeCell;
use std::task::{RawWaker, RawWakerVTable, Waker};

/// A waker for single producer and single consumer.
///
/// The difference between this `AtomicWaker` and the `futures` one
/// is, there is only contention between register and waker in this `AtomicWaker`,
/// but the `futures` implementation also has to consider the contention between the registers.
pub(crate) struct AtomicWaker {
    stat: AtomicU8,
    waker: UnsafeCell<Waker>,
}

unsafe impl Send for AtomicWaker {}

const WAITING: u8 = 0b0;
const WAKING: u8 = 0b01;
const REGISTERING: u8 = 0b10;
const FULL: u8 = WAKING | REGISTERING;

impl AtomicWaker {
    pub(crate) fn register(&self, waker: &Waker) {
        match self.stat.fetch_or(REGISTERING, Ordering::AcqRel) {
            WAKING => {
                // the waker is waking now, we just wake this waker.
                waker.wake_by_ref();
                self.stat.fetch_and(!REGISTERING, Ordering::Release);
            }
            state => {
                debug_assert_eq!(state, WAITING);

                // Safety: there is no waker operate on this cell,
                // so we can just replace the inner waker.
                unsafe {
                    let this = &mut *self.waker.get();
                    if !this.will_wake(waker) {
                        *this = waker.clone();
                    }
                }

                match self.stat.fetch_and(!REGISTERING, Ordering::AcqRel) {
                    FULL => {
                        // the wake method was called during execution of upon statements,
                        // so we need to wake up the waker.
                        unsafe {
                            (&*self.waker.get()).wake_by_ref();
                        }
                        self.stat.store(WAITING, Ordering::Release);
                    }
                    state => {
                        debug_assert_eq!(state, REGISTERING)
                    }
                }
            }
        }
    }

    pub(crate) fn wake_by_ref(&self) {
        match self.stat.fetch_or(WAKING, Ordering::AcqRel) {
            WAITING => {
                // Safety: we hold the WAKING bit, so there is no
                // register operating on the cell.
                unsafe {
                    (&*self.waker.get()).wake_by_ref();
                }
                self.stat.fetch_and(!WAKING, Ordering::Release);
            }
            state => {
                // fail to hold the WAKING bit, we just let the register call wake method.
                debug_assert!(state == REGISTERING || state == FULL || state == WAKING);
            }
        }
    }
}

impl Default for AtomicWaker {
    fn default() -> Self {
        Self {
            stat: AtomicU8::new(WAITING),
            waker: UnsafeCell::new(dummy_waker()),
        }
    }
}

const NOOP_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);

unsafe fn noop_clone(_: *const ()) -> RawWaker {
    RawWaker::new(ptr::null(), &NOOP_WAKER_VTABLE)
}

unsafe fn noop(_: *const ()) {}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(ptr::null(), &NOOP_WAKER_VTABLE)) }
}
