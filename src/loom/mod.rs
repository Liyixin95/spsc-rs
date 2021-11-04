#[cfg(loom)]
pub(crate) use loom::sync::atomic::AtomicU8;
#[cfg(not(loom))]
pub(crate) use std::sync::atomic::AtomicU8;

#[cfg(loom)]
pub(crate) use loom::sync::atomic::AtomicBool;
#[cfg(not(loom))]
pub(crate) use std::sync::atomic::AtomicBool;

#[cfg(loom)]
pub(crate) use loom::sync::atomic::AtomicUsize;
#[cfg(not(loom))]
pub(crate) use std::sync::atomic::AtomicUsize;

#[cfg(loom)]
pub(crate) use loom::sync::atomic::Ordering;
#[cfg(not(loom))]
pub(crate) use std::sync::atomic::Ordering;

#[cfg(loom)]
pub(crate) use loom::cell::UnsafeCell;
#[cfg(not(loom))]
mod cell;
#[cfg(not(loom))]
pub(crate) use self::cell::UnsafeCell;
