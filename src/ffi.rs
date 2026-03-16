use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

use crate::PAGE_ALLOCATOR;
use crate::heap::block::ALIGN;

const DEFAULT_LAYOUT: Layout = unsafe { Layout::from_size_align_unchecked(1, ALIGN) };

#[unsafe(no_mangle)]
pub extern "C" fn tal_alloc(size: usize) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let layout = unsafe { Layout::from_size_align_unchecked(size, ALIGN) };
    unsafe { PAGE_ALLOCATOR.alloc(layout) as *mut c_void }
}

#[unsafe(no_mangle)]
pub extern "C" fn tal_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe { PAGE_ALLOCATOR.dealloc(ptr as *mut u8, DEFAULT_LAYOUT) }
}

#[unsafe(no_mangle)]
pub extern "C" fn tal_mem_show() {
    PAGE_ALLOCATOR.mem_show();
}

#[unsafe(no_mangle)]
pub extern "C" fn tal_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    if ptr.is_null() {
        return tal_alloc(new_size);
    }
    if new_size == 0 {
        tal_free(ptr);
        return core::ptr::null_mut();
    }
    unsafe { PAGE_ALLOCATOR.realloc(ptr as *mut u8, DEFAULT_LAYOUT, new_size) as *mut c_void }
}
