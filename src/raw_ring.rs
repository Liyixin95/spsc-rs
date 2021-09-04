use std::mem::MaybeUninit;

pub struct Ring<T> {
    array: Box<[MaybeUninit<T>]>,
    capacity: usize,
    read: usize,
    write: usize,
}

impl<T> Ring<T> {
    pub fn with_capacity(size: usize) -> Self {
        let array = (0..size)
            .map(|_| MaybeUninit::uninit())
            .collect();
        Self {
            array,
            capacity: size,
            read: 0,
            write: 0
        }
    }

    pub fn next(&mut self) -> Option<usize> {
        self.write += 1;
        let warp_point = self.write - self.capacity;
        if warp_point > self.read {
            Some(self.write & (self.capacity - 1))
        } else {
            return None
        }
    }

    pub unsafe fn push(&mut self, pos: usize, t: T) {
        let ptr = self.array.get_unchecked_mut(pos);
        *ptr = MaybeUninit::new(t);
    }

    pub fn try_pop(&mut self) -> Option<T> {
        if self.read == self.write {
            return None;
        } else {
            self.read += 1;
            unsafe {
                let ptr = self.array.get_unchecked(self.read & (self.capacity - 1)).as_ptr();
                Some(ptr.read())
            }
        }
    }
}