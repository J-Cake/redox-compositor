use std::mem;
use std::alloc::{self, Layout, alloc};
use syscall::PAGE_SIZE;

pub fn aligned_vec(width: i32, height: i32) -> Vec<u32> {
    #[repr(C, align(4096))]
    #[derive(Clone)]
    struct Vector([u32; PAGE_SIZE / mem::size_of::<u32>()]);

    let pages = ((width * height) as usize * mem::size_of::<u32>()) / mem::size_of::<Vector>() + 1;
    let mut vec = vec![Vector([0xffaaaaaa; PAGE_SIZE / mem::size_of::<u32>()]); pages];

    let ptr = vec.as_mut_ptr() as *mut u32;
    let len = (width * height) as usize;
    mem::forget(vec);

    unsafe { Vec::from_raw_parts(ptr, len, len) }
}

pub fn aligned_vec_new(width: i32, height: i32) -> Vec<u32> {
    unsafe { alloc_vec::<u32>((width * height) as usize) }
}

pub unsafe fn alloc_vec<T>(len: usize) -> Vec<T> {
    let bytes = len * mem::size_of::<T>() + (PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let layout = Layout::from_size_align(bytes, PAGE_SIZE)
        .unwrap();
    let ptr = alloc(layout);
    Vec::from_raw_parts(ptr as *mut T, len, len)
}
