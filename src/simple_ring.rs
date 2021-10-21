use crate::raw_ring::RawRing;
use crate::Ring;

pub struct SimpleRing<T> {
    buf: RawRing<T>,
}

impl<T> Ring<T> for SimpleRing<T> {
    fn next_idx(&self) -> Option<usize> {
        if self.buf.capacity() - self.buf.len() == 1 {
            None
        } else {
            let next = self.buf.producer_pos();
            Some(self.buf.index(next))
        }
    }

    fn try_pop(&self) -> Option<T> {
        if self.buf.is_empty() {
            return None;
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

impl<T> SimpleRing<T> {
    pub fn with_capacity(size: usize) -> Self {
        let buf = RawRing::with_capacity(size);
        Self { buf }
    }
}
