use crate::loom::UnsafeCell;
use crate::loom::{AtomicU8, Ordering};
use core::ptr;
use std::task::{RawWaker, RawWakerVTable, Waker};

/// A waker for single producer and single consumer.
///
/// The difference between this `AtomicWaker` and the `futures` one
/// is, there is only contention between register and waker in this `AtomicWaker`,
/// but the `futures` implementation also has to consider the contention between the registers.
pub(crate) struct AtomicWaker {
    state: AtomicU8,
    waker: UnsafeCell<Waker>,
}

unsafe impl Send for AtomicWaker {}

const WAITING: u8 = 0b0;
const WAKING: u8 = 0b01;
const REGISTERING: u8 = 0b10;
const FULL: u8 = WAKING | REGISTERING;

impl AtomicWaker {
    pub(crate) fn register(&self, waker: &Waker) {
        match self.state.fetch_or(REGISTERING, Ordering::AcqRel) {
            WAKING => {
                // the waker is waking now, we just wake this waker.
                waker.wake_by_ref();

                self.state.fetch_and(!REGISTERING, Ordering::Release);

                // loom's scheduler is not fair, so we need to manually yield in loom
                // to avoid infinitely wakeup.
                //
                // see https://docs.rs/loom/0.5.2/loom/#yielding
                #[cfg(loom)]
                loom::thread::yield_now();
            }
            state => {
                debug_assert_eq!(state, WAITING);

                self.waker.with_mut(|ptr| {
                    // Safety: there is no waker operate on this cell,
                    // so we can just replace the inner waker.
                    let inner_waker = unsafe { &mut *ptr };
                    if !inner_waker.will_wake(waker) {
                        *inner_waker = waker.clone();
                    }
                });

                match self.state.fetch_and(!REGISTERING, Ordering::AcqRel) {
                    FULL => {
                        // the wake method was called during execution of upon statements,
                        // so we need to wake up the waker.
                        self.waker.with(|ptr| {
                            let inner_waker = unsafe { &*ptr };
                            inner_waker.wake_by_ref();
                        });

                        // We must swap the WAITING status into state, rather than just store.
                        // Consider this situation:
                        //                                register waker
                        // receiver:  (first fetch -----------> second fetch  ---------> store) -> read
                        // sender  :                fetch,set ---------------> fetch
                        //
                        // in this situation, the last read don't have synchronize-with relationship with
                        // the first set, so it may not read the latest value.
                        //
                        // But if we use swap (for Acquire Ordering), then the synchronize-with relationship
                        // can be established, making that read be aware of the latest value.
                        self.state.swap(WAITING, Ordering::AcqRel);
                    }
                    state => {
                        debug_assert_eq!(state, REGISTERING)
                    }
                }
            }
        }
    }

    pub(crate) fn wake_by_ref(&self) {
        match self.state.fetch_or(WAKING, Ordering::AcqRel) {
            WAITING => {
                self.waker.with(|ptr| {
                    // Safety: we hold the WAKING bit, so there is no
                    // register operating on the cell.
                    let inner_waker = unsafe { &*ptr };
                    inner_waker.wake_by_ref();
                });

                self.state.fetch_and(!WAKING, Ordering::Release);
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
            state: AtomicU8::new(WAITING),
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
