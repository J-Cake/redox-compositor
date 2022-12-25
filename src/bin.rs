use std::mem;
use syscall::PAGE_SIZE;

pub fn aligned_vec(width: u32, height: u32) -> Vec<u32> {
    #[repr(C, align(4096))]
    #[derive(Clone)]
    struct Vector([u32; PAGE_SIZE / mem::size_of::<u32>()]);

    let pages = ((width * height) as usize * mem::size_of::<u32>()) / mem::size_of::<Vector>() + 1;
    let mut vec = vec![Vector([0xffaaaaaa; PAGE_SIZE / mem::size_of::<u32>()]); pages];

    let ptr = vec.as_mut_ptr() as *mut u32;
    let len = (width * height) as usize;// * mem::size_of::<u32>();
    mem::forget(vec);

    unsafe { Vec::from_raw_parts(ptr, len, len) }
}
