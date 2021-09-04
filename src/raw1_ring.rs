use std::mem::MaybeUninit;

pub(crate) struct RawRing<T> {
    buf: Box<[MaybeUninit<T>]>,
    cap: usize,
    mask: usize,
    pub(crate) producer_pos: usize,
    pub(crate) consumer_pos: usize,
}

impl<T> RawRing<T> {
    pub(crate) fn with_capacity(cap: usize) -> Self {
        let cap = cap.checked_next_power_of_two().expect("capacity overflow");
        let buf = (0..cap)
            .map(|_| MaybeUninit::uninit())
            .collect();
        Self {
            buf,
            consumer_pos: 0,
            producer_pos: 0,
            mask: cap - 1,
            cap
        }
    }

    pub(crate) fn producer_idx(&self) -> usize {
        self.index(self.producer_pos)
    }

    pub(crate) fn consumer_idx(&self) -> usize {
        self.index(self.consumer_pos)
    }

    pub(crate) fn index(&self, pos: usize) -> usize {
        pos & self.mask
    }

    pub(crate) fn capacity(&self) -> usize {
        self.cap
    }

    pub(crate) unsafe fn buffer_read(&mut self, off: usize) -> T {
        unsafe {
            let ptr = self.buf.as_ptr().add(off);
            ptr.read().assume_init()
        }
    }

    pub(crate) unsafe fn buffer_write(&mut self, off: usize, value: T) {
        unsafe {
            let ptr = self.buf.as_mut_ptr().add(off);
            ptr.write(MaybeUninit::new(value))
        }
    }
}