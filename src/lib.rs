pub mod raw_ring;
pub mod ptr_ring;
pub(crate) mod raw1_ring;
pub mod spsc;

pub(crate) trait Ring<T> {
    fn next(&mut self) -> Option<usize>;

    fn try_pop(&mut self) ->Option<T>;

    unsafe fn set(&mut self, t: T, pos: usize);
}

