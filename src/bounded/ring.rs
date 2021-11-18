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

unsafe impl<T: Send, I: Send + Indexer> Send for Ring<T, I> {}

unsafe impl<T: Send, I: Send + Indexer> Sync for Ring<T, I> {}

pub trait Indexer {
    fn index(&self, pos: usize) -> usize;

    fn cap(&self) -> usize;
}

pub struct And {
    mask: usize,
    cap: usize,
}

impl Indexer for And {
    fn index(&self, pos: usize) -> usize {
        pos & self.mask
    }

    fn cap(&self) -> usize {
        self.cap
    }
}

pub struct Remainder {
    cap: usize,
}

impl Indexer for Remainder {
    fn index(&self, pos: usize) -> usize {
        pos % self.cap
    }

    fn cap(&self) -> usize {
        self.cap
    }
}

pub(crate) type ExactRing<T> = Ring<T, Remainder>;
pub(crate) type P2Ring<T> = Ring<T, And>;

pub(crate) struct Ring<T, I: Indexer> {
    buf: Box<[UnsafeCell<MaybeUninit<T>>]>,
    indexer: I,
    producer_pos: AtomicPos,
    consumer_pos: AtomicPos,
}

impl<T> Ring<T, And> {
    pub(crate) fn with_capacity(cap: usize) -> Self {
        let cap = cmp::max(cap + 1, 2)
            .checked_next_power_of_two()
            .expect("capacity overflow");

        let indexer = And { mask: cap - 1, cap };

        Self::new(indexer)
    }
}

impl<T> Ring<T, Remainder> {
    pub(crate) fn with_capacity(cap: usize) -> Self {
        let indexer = Remainder {
            cap: cmp::max(cap + 1, 2),
        };
        Self::new(indexer)
    }
}

impl<T, I: Indexer> Drop for Ring<T, I> {
    fn drop(&mut self) {
        unsafe {
            let (left, right) = self.as_mut_slice();
            std::ptr::drop_in_place(right);
            std::ptr::drop_in_place(left);
        }
    }
}

impl<T, I: Indexer> Ring<T, I> {
    fn new(indexer: I) -> Self {
        let buf = (0..indexer.cap())
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect();
        Self {
            buf,
            consumer_pos: Default::default(),
            producer_pos: Default::default(),
            indexer,
        }
    }

    pub(crate) fn is_full(&self) -> bool {
        self.capacity() - self.len() == 1
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.consumer_pos() == self.producer_pos()
    }

    pub(crate) fn next_idx(&self) -> Option<usize> {
        if self.is_full() {
            None
        } else {
            let next = self.producer_pos();
            Some(self.index(next))
        }
    }

    pub(crate) fn try_pop(&self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            unsafe {
                let now = self.consumer_pos.load(Ordering::Acquire);
                let idx = self.index(now);
                self.consumer_pos.store(now + 1, Ordering::Release);
                Some(self.buffer_read(idx))
            }
        }
    }

    pub(crate) unsafe fn set_unchecked(&self, t: T, idx: usize) {
        self.buffer_write(idx, t);
        let now = self.producer_pos.load(Ordering::Acquire);
        self.producer_pos.store(now + 1, Ordering::Release);
    }

    fn len(&self) -> usize {
        let diff = self.producer_pos().wrapping_sub(self.consumer_pos());
        self.index(diff)
    }

    fn producer_idx(&self) -> usize {
        self.index(self.producer_pos())
    }

    fn consumer_idx(&self) -> usize {
        self.index(self.consumer_pos())
    }

    fn index(&self, pos: usize) -> usize {
        self.indexer.index(pos)
    }

    pub(crate) fn capacity(&self) -> usize {
        self.indexer.cap()
    }

    unsafe fn buffer_read(&self, idx: usize) -> T {
        let ptr = self.buf.as_ptr();
        let ptr = &*ptr.add(idx);
        ptr.with(|inner| inner.read().assume_init())
    }

    unsafe fn buffer_write(&self, idx: usize, value: T) {
        let cell = self.buf.get_unchecked(idx);
        cell.with_mut(|ptr| ptr.write(MaybeUninit::new(value)));
    }

    fn consumer_pos(&self) -> usize {
        self.consumer_pos.load(Ordering::Acquire)
    }

    fn producer_pos(&self) -> usize {
        self.producer_pos.load(Ordering::Acquire)
    }

    unsafe fn as_mut_slice(&mut self) -> (&mut [T], &mut [T]) {
        let contiguous = self.consumer_pos() <= self.producer_pos();
        let ptr: *mut T = self.buf.as_mut_ptr().cast();
        if contiguous {
            let empty = from_raw_parts_mut(ptr, 0);
            let len = self.producer_pos() - self.consumer_pos() + 1;
            let buf = from_raw_parts_mut(ptr.add(self.consumer_pos()), len);
            (buf, empty)
        } else {
            let p_idx = self.producer_idx();
            let c_idx = self.consumer_idx();
            let left = from_raw_parts_mut(ptr, p_idx);
            let right = from_raw_parts_mut(ptr.add(c_idx), self.capacity() - c_idx);
            (left, right)
        }
    }
}
