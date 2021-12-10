use crate::loom::{AtomicPtr, AtomicUsize, Ordering, UnsafeCell};
use std::mem::MaybeUninit;
use std::ptr::{null_mut, NonNull};

#[cfg(not(loom))]
const BLOCK_SIZE: usize = 128;

#[cfg(loom)]
const BLOCK_SIZE: usize = 2;

const BLOCK_MASK: usize = BLOCK_SIZE - 1;

struct Block<T> {
    array: [UnsafeCell<MaybeUninit<T>>; BLOCK_SIZE],
    next: AtomicPtr<Block<T>>,
}

impl<T> Block<T> {
    fn new_uninit() -> Self {
        let mut array = MaybeUninit::uninit();
        if_loom! {
            let p = array.as_mut_ptr() as *mut UnsafeCell<MaybeUninit<T>>;
            for i in 0..BLOCK_SIZE {
                unsafe {
                    p.add(i).write(UnsafeCell::new(MaybeUninit::uninit()));
                }
            }
        }

        let array = unsafe { array.assume_init() };
        Self {
            array,
            next: AtomicPtr::new(null_mut()),
        }
    }

    unsafe fn read(&self, slot_idx: usize) -> T {
        self.array
            .get_unchecked(slot_idx)
            .with(|inner| inner.read().assume_init())
    }

    unsafe fn write(&self, t: T, slot_idx: usize) {
        self.array
            .get_unchecked(slot_idx)
            .with_mut(|inner| inner.write(MaybeUninit::new(t)));
    }

    unsafe fn load_next_unchecked(&self) -> NonNull<Block<T>> {
        NonNull::new_unchecked(self.next.load(Ordering::Acquire))
    }
}

pub(crate) struct Queue<T> {
    producer: UnsafeCell<NonNull<Block<T>>>,
    producer_pos: AtomicUsize,
    consumer: UnsafeCell<NonNull<Block<T>>>,
    consumer_pos: AtomicUsize,
}

unsafe impl<T: Send> Send for Queue<T> {}

unsafe impl<T: Send> Sync for Queue<T> {}

impl<T> Queue<T> {
    pub(crate) fn new() -> Self {
        let block = Box::new(Block::new_uninit());
        let block_ptr = Box::into_raw(block);
        Self {
            producer: UnsafeCell::new(NonNull::new(block_ptr).unwrap()),
            producer_pos: AtomicUsize::new(0),
            consumer: UnsafeCell::new(NonNull::new(block_ptr).unwrap()),
            consumer_pos: AtomicUsize::new(0),
        }
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        unsafe {
            while self.try_pop().is_some() {}

            // drop the last block
            self.consumer.with(|ptr| {
                std::mem::drop(Box::from_raw((*ptr).as_ptr()));
            })
        }
    }
}

impl<T> Queue<T> {
    pub(crate) fn is_empty(&self) -> bool {
        self.producer_pos.load(Ordering::Acquire) == self.consumer_pos.load(Ordering::Acquire)
    }

    pub(crate) unsafe fn push(&self, t: T) {
        let now = self.producer_pos.load(Ordering::Acquire);
        let now_idx = now & BLOCK_MASK;
        let next = now + 1;

        self.producer.with(|ptr| unsafe {
            (*ptr).as_ref().write(t, now_idx);
        });

        if (next & BLOCK_MASK) < now_idx {
            let next_ptr = Box::into_raw(Box::new(Block::new_uninit()));
            self.producer.with_mut(|ptr| unsafe {
                let refs = (*ptr).as_mut();
                refs.next.store(next_ptr, Ordering::Release);
                *ptr = NonNull::new_unchecked(next_ptr);
            });
        }

        self.producer_pos.store(next, Ordering::Release);
    }

    pub(crate) unsafe fn try_pop(&self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let now = self.consumer_pos.load(Ordering::Acquire);
            let now_idx = now & BLOCK_MASK;
            let next = now + 1;

            let ret = self.consumer.with(|ptr| (*ptr).as_ref().read(now_idx));

            if (next & BLOCK_MASK) < now_idx {
                self.consumer.with_mut(|ptr| {
                    let refs = (*ptr).as_ref();
                    let old = *ptr;
                    *ptr = refs.load_next_unchecked();

                    std::mem::drop(Box::from_raw(old.as_ptr()));
                })
            }

            self.consumer_pos.store(next, Ordering::Release);
            Some(ret)
        }
    }
}

#[cfg(all(test, loom))]
mod tests {
    use crate::unbounded::queue::Queue;
    use loom::sync::Arc;

    #[cfg(loom)]
    #[test]
    fn push_pop() {
        loom::model(|| {
            let queue = Arc::new(Queue::new());
            let queue1 = queue.clone();
            loom::thread::spawn(move || unsafe {
                for i in 0..3 {
                    queue1.push(i);
                }
            });

            let mut count = 0;
            loop {
                match unsafe { queue.try_pop() } {
                    None => loom::thread::yield_now(),
                    Some(idx) => {
                        assert_eq!(count, idx);
                        count += 1;
                        if count == 3 {
                            break;
                        }
                    }
                }
            }
        })
    }
}
