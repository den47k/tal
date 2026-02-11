use core::ptr;

use crate::{
    PAGE_SIZE,
    heap::block::{BUSY, BlockHeader, FIRST, LAST, align_up},
    os,
};

const DEFAULT_ARENA_PAGES: usize = 8;

pub fn default_arena_size() -> usize {
    DEFAULT_ARENA_PAGES * *PAGE_SIZE
}

pub fn create_default_arena() -> *mut BlockHeader {
    unsafe {
        let len = default_arena_size();
        let base = os::map(len);
        if base.is_null() {
            return ptr::null_mut();
        }

        let b = base as *mut BlockHeader;
        (*b).prev_size = 0;
        (*b).set_size_and_flags(len, FIRST | LAST);

        b
    }
}

pub fn create_large_arena(needed_block_bytes: usize) -> *mut BlockHeader {
    unsafe {
        let len = align_up(needed_block_bytes, *PAGE_SIZE);
        let base = os::map(len);
        if base.is_null() {
            return ptr::null_mut();
        }

        let b = base as *mut BlockHeader;
        (*b).prev_size = 0;
        (*b).set_size_and_flags(len, BUSY | FIRST | LAST);
        b
    }
}

pub unsafe fn destroy_arena(base: *mut u8, len: usize) {
    os::unmap(base, len);
}
