use core::ptr;

use crate::{
    PAGE_SIZE,
    heap::block::{BUSY, BlockHeader, FIRST, LAST, align_up},
    os,
};

const DEFAULT_ARENA_PAGES: usize = 8;
const FALLBACK_ARENA_SIZE: usize = 32 * 1024;

pub fn default_arena_size() -> usize {
    let page = *PAGE_SIZE;
    if page == 0 {
        FALLBACK_ARENA_SIZE
    } else {
        DEFAULT_ARENA_PAGES * page
    }
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
        let page = *PAGE_SIZE;
        let len = if page == 0 {
            needed_block_bytes
        } else {
            align_up(needed_block_bytes, page)
        };
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

        let meta_end = start + crate::heap::block::FREE_META_SIZE;

        let advise_start = page_align_up(meta_end, page);
        let advise_end = page_align_down(end, page);

        if advise_end > advise_start {
            os::madvise(advise_start as *mut u8, advise_end - advise_start);
        }
    }
}
