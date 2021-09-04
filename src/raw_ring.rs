use std::mem::MaybeUninit;
use std::sync::atomic::AtomicUsize;

pub struct SimpleRing<T> {
    array: Box<[MaybeUninit<T>]>,
    capacity: usize,
    mask: usize,
    consumer_pos: usize,
    producer_pos: usize,
}

impl<T> Drop for SimpleRing<T> {
    fn drop(&mut self) {
        while let Some(t) = self.try_pop() {
            std::mem::drop(t)
        }
    }
}

impl<T> SimpleRing<T> {
    pub fn with_capacity(size: usize) -> Self {
        let array = (0..size)
            .map(|_| MaybeUninit::uninit())
            .collect();
        let capacity = size.checked_next_power_of_two().expect("invalid capacity");
        Self {
            array,
            capacity,
            mask: capacity - 1,
            consumer_pos: 0,
            producer_pos: 0,
        }
    }

    pub fn next(&mut self) -> Option<usize> {
        let next = self.producer_pos.overflowing_add(1).0;
        self.producer_pos = next;
        let warp_point = next.overflowing_sub(self.capacity).0;
        if warp_point >= self.consumer_pos {
            Some(next & self.mask)
        } else {
            return None
        }
    }

    pub(crate) unsafe fn set(&mut self, pos: usize, t: T) {
        let ptr = self.array.get_unchecked_mut(pos);
        *ptr = MaybeUninit::new(t);
    }

    pub(crate) fn try_pop(&mut self) -> Option<T> {
        let read = self.consumer_pos;
        if read == self.producer_pos {
            return None;
        } else {
            unsafe {
                let pos = read.overflowing_add(1).0;
                self.consumer_pos = pos;
                let ptr = self.array.get_unchecked(pos & self.mask).as_ptr();
                Some(ptr.read())
            }
        }
    }
}