mod bounded_ring;
mod raw_ring;
pub(crate) use self::bounded_ring::BoundedRing;

pub trait Ring<T> {
    fn next_idx(&self) -> Option<usize>;

    fn try_pop(&self) -> Option<T>;

    unsafe fn set(&self, t: T, idx: usize);
}
