use super::raw_ring::RawRing;
use super::Ring;

pub struct BoundedRing<T> {
    buf: RawRing<T>,
}

impl<T> Ring<T> for BoundedRing<T> {
    fn is_empty(&self) -> bool {
        self.buf.len() == 0
    }

    fn is_full(&self) -> bool {
        self.buf.capacity() - self.buf.len() == 1
    }

    fn next_idx(&self) -> Option<usize> {
        if self.is_full() {
            None
        } else {
            let next = self.buf.producer_pos();
            Some(self.buf.index(next))
        }
    }

    fn try_pop(&self) -> Option<T> {
        if self.buf.is_empty() {
            None
        } else {
            unsafe {
                let idx = self.buf.consumer_idx();
                self.buf.next_consumer_pos(1);
                Some(self.buf.buffer_read(idx))
            }
        }
    }

    unsafe fn set(&self, t: T, idx: usize) {
        self.buf.buffer_write(idx, t);
        self.buf.next_producer_pos(1);
    }
}

impl<T> BoundedRing<T> {
    pub fn with_capacity(size: usize) -> Self {
        let buf = RawRing::with_capacity(size);
        Self { buf }
    }
}
