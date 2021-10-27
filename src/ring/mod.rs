mod bounded_ring;
mod raw_ring;
pub(crate) use self::bounded_ring::BoundedRing;

pub trait Ring<T> {
    fn is_empty(&self) -> bool;

    fn is_full(&self) -> bool;

    fn next_idx(&self) -> Option<usize>;

    fn try_pop(&self) -> Option<T>;

    unsafe fn set(&self, t: T, idx: usize);
}
