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

#[inline]
fn page_align_up(x: usize, page: usize) -> usize {
    debug_assert!(page.is_power_of_two());
    (x + page - 1) & !(page - 1)
}

#[inline]
fn page_align_down(x: usize, page: usize) -> usize {
    debug_assert!(page.is_power_of_two());
    x & !(page - 1)
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

pub unsafe fn advise_free_pages(b: *mut BlockHeader) {
    unsafe {
        let page = *PAGE_SIZE;
        if page == 0 {
            return;
        }

        let start = b as usize;
        let end = start + (*b).size();

        // Do NOT discard header+AVL metadata (it must remain intact)
        let meta_end = start + crate::heap::block::FREE_META_SIZE;

        // Advise only full pages completely inside the free block and after metadata.
        let advise_start = page_align_up(meta_end, page);
        let advise_end = page_align_down(end, page);

        if advise_end > advise_start {
            os::madvise(advise_start as *mut u8, advise_end - advise_start);
        }
    }
}
