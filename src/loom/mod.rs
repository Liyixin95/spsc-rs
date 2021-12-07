#[cfg(loom)]
pub(crate) use loom::sync::atomic::*;
#[cfg(not(loom))]
pub(crate) use std::sync::atomic::*;

#[cfg(loom)]
pub(crate) use loom::sync::Arc;
#[cfg(not(loom))]
pub(crate) use std::sync::Arc;

#[cfg(loom)]
pub(crate) use loom::cell::UnsafeCell;

#[cfg(not(loom))]
mod cell;

#[cfg(not(loom))]
pub(crate) use self::cell::UnsafeCell;

#[cfg(loom)]
pub(crate) use loom::thread;
#[cfg(not(loom))]
pub(crate) use std::thread;

macro_rules! if_loom {
    ($($t:tt)*) => {{
        #[cfg(loom)]
        const LOOM: bool = true;
        #[cfg(not(loom))]
        const LOOM: bool = false;

        if LOOM {
            $($t)*
        }
    }}
}
