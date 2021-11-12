use self::inner::AtomicPos;
use crate::loom::UnsafeCell;
use core::cmp;
use std::mem::MaybeUninit;
use std::slice::from_raw_parts_mut;
use std::sync::atomic::Ordering;

#[cfg(feature = "cache-padded")]
mod inner {
    use crate::loom::AtomicUsize;
    use cache_padded::CachePadded;
    use core::ops::Deref;

    #[derive(Default)]
    pub(crate) struct AtomicPos {
        inner: CachePadded<AtomicUsize>,
    }

    impl Deref for AtomicPos {
        type Target = AtomicUsize;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }
}

#[cfg(not(feature = "cache-padded"))]
mod inner {
    use crate::loom::AtomicUsize;
    use core::ops::Deref;

    #[derive(Default)]
    pub(crate) struct AtomicPos {
        inner: AtomicUsize,
    }

    impl Deref for AtomicPos {
        type Target = AtomicUsize;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }
}

unsafe impl<T: Send> Send for RawRing<T> {}
unsafe impl<T: Send> Sync for RawRing<T> {}

pub(crate) struct RawRing<T> {
    buf: Box<[UnsafeCell<MaybeUninit<T>>]>,
    cap: usize,
    mask: usize,
    producer_pos: AtomicPos,
    consumer_pos: AtomicPos,
}

impl<T> Drop for RawRing<T> {
    fn drop(&mut self) {
        unsafe {
            let (left, right) = self.as_mut_slice();
            std::ptr::drop_in_place(right);
            std::ptr::drop_in_place(left);
        }
    }
}

impl<T> RawRing<T> {
    pub(crate) fn with_capacity(cap: usize) -> Self {
        let cap = cmp::max(cap + 1, 2)
            .checked_next_power_of_two()
            .expect("capacity overflow");

        let buf = (0..cap)
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect();
        Self {
            buf,
            consumer_pos: Default::default(),
            producer_pos: Default::default(),
            mask: cap - 1,
            cap,
        }
    }

    pub(crate) fn len(&self) -> usize {
        let diff = self.producer_pos().wrapping_sub(self.consumer_pos());
        self.index(diff)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.consumer_pos() == self.producer_pos()
    }

    pub(crate) fn producer_idx(&self) -> usize {
        self.index(self.producer_pos())
    }

    pub(crate) fn consumer_idx(&self) -> usize {
        self.index(self.consumer_pos())
    }

    pub(crate) fn index(&self, pos: usize) -> usize {
        pos & self.mask
    }

    pub(crate) fn capacity(&self) -> usize {
        self.cap
    }

    pub(crate) unsafe fn buffer_read(&self, idx: usize) -> T {
        let ptr = self.buf.as_ptr();
        let ptr = &*ptr.add(idx);
        ptr.with(|inner| inner.read().assume_init())
    }

    pub(crate) unsafe fn buffer_write(&self, idx: usize, value: T) {
        let cell = self.buf.get_unchecked(idx);
        cell.with_mut(|ptr| ptr.write(MaybeUninit::new(value)));
    }

    pub(crate) fn next_producer_pos(&self, off: usize) -> usize {
        let now = self.producer_pos.load(Ordering::Acquire);
        let next = now + off;
        self.producer_pos.store(next, Ordering::Release);
        next
    }

    pub(crate) fn next_consumer_pos(&self, off: usize) -> usize {
        let now = self.consumer_pos.load(Ordering::Acquire);
        let next = now + off;
        self.consumer_pos.store(next, Ordering::Release);
        next
    }

    pub(crate) fn consumer_pos(&self) -> usize {
        self.consumer_pos.load(Ordering::Acquire)
    }

    pub(crate) fn producer_pos(&self) -> usize {
        self.producer_pos.load(Ordering::Acquire)
    }

    unsafe fn as_mut_ptr(&mut self) -> *mut T {
        self.buf.as_mut_ptr().cast()
    }

    unsafe fn as_mut_slice(&mut self) -> (&mut [T], &mut [T]) {
        let contiguous = self.consumer_pos() <= self.producer_pos();
        let ptr = self.as_mut_ptr();
        if contiguous {
            let empty = from_raw_parts_mut(ptr, 0);
            let len = self.producer_pos() - self.consumer_pos() + 1;
            let buf = from_raw_parts_mut(ptr.add(self.consumer_pos()), len);
            (buf, empty)
        } else {
            let p_idx = self.producer_idx();
            let c_idx = self.consumer_idx();
            let left = from_raw_parts_mut(ptr, p_idx);
            let right = from_raw_parts_mut(ptr.add(c_idx), self.cap - c_idx);
            (left, right)
        }
    }
}
