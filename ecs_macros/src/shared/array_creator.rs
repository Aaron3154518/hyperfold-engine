// Memory management for array creation
pub struct ArrayCreator<T, const N: usize> {
    ptr: *mut T,
    len: usize,
}

impl<T, const N: usize> ArrayCreator<T, N> {
    pub fn create<F>(f: F) -> [T; N]
    where
        F: Fn(usize) -> T,
    {
        let mut arr: core::mem::MaybeUninit<[T; N]> = core::mem::MaybeUninit::uninit();
        let mut ac = Self {
            ptr: arr.as_mut_ptr() as *mut T,
            len: 0,
        };
        for i in 0..N {
            ac.alloc(f(i));
        }
        unsafe { arr.assume_init() }
    }

    fn alloc(&mut self, t: T) {
        assert!(self.len < N);
        unsafe { self.ptr.add(self.len).write(t) };
        self.len += 1;
    }
}

impl<T, const N: usize> Drop for ArrayCreator<T, N> {
    fn drop(self: &'_ mut Self) {
        unsafe {
            core::ptr::drop_in_place(core::slice::from_raw_parts_mut(self.ptr, self.len));
        }
    }
}
