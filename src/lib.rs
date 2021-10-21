mod atomic_waker;
pub mod error;
mod raw_ring;
mod simple_ring;
pub mod spsc;

pub trait Ring<T> {
    fn next_idx(&self) -> Option<usize>;

    fn try_pop(&self) -> Option<T>;

    unsafe fn set(&self, t: T, idx: usize);
}
