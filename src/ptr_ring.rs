use std::sync::atomic::{AtomicPtr, Ordering, AtomicU8, AtomicBool};
use crate::raw1_ring::RawRing;

struct Tag<T> {
    inner: T,
    tag: AtomicBool,
}

impl<T> Tag<T> {
    fn vacant(&self) -> bool {
        !self.tag.load(Ordering::Relaxed)
    }

    fn occupy(&mut self)  {
        self.tag.store(true, Ordering::Relaxed)
    }
}


pub struct PtrRing<T> {
    buf: RawRing<Tag<T>>,
    limit: usize,
    step: usize,
}

impl<T> PtrRing<T> {
    pub fn with_capacity(cap: usize) -> Self {
        let buf = RawRing::with_capacity(cap);
        Self {
            step: (buf.capacity() / 4).min(4),
            buf,
            limit: 0,
        }
    }

    pub fn next(&mut self) -> Option<usize> {
        let buf = &mut self.buf;
        return if self.limit > 0 {
            self.limit -= 1;
            buf.producer_pos += 1;
            Some(buf.producer_idx())
        } else {
            unsafe {
                let idx = buf.index(buf.producer_pos + self.step);
                if buf.buffer_read(idx).vacant() {
                    self.limit = self.step - 1;
                    buf.producer_pos += 1;
                    Some(buf.producer_idx())
                } else if buf.buffer_read(buf.producer_pos + 1).vacant() {
                    buf.producer_pos += 1;
                    Some(buf.producer_idx())
                } else {
                    None
                }
            }
        }
    }

    pub(crate) unsafe fn set(&self, t: T, idx: usize)  {
    }
}