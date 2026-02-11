use core::alloc::{GlobalAlloc, Layout};
use core::cmp::max;
use core::ptr;

use crate::free::FreeTree;
use crate::heap::arena::{self, default_arena_size};
use crate::heap::block::{
    ALIGN, BUSY, BlockHeader, FIRST, HEADER_SIZE, LAST, align_up, header_from_payload,
    min_free_block_size, next_block, payload_ptr, prev_block,
};
use crate::os;
use crate::sync::SpinLock;

#[derive(Default)]
struct AllocState {
    free: FreeTree,
}

impl AllocState {
    pub const fn new() -> Self {
        Self {
            free: FreeTree::new(),
        }
    }
}

static STATE: SpinLock<AllocState> = SpinLock::new(AllocState::new());

pub struct ArenaAllocator;

impl ArenaAllocator {
    unsafe fn add_default_arena(state: &mut AllocState) -> bool {
        unsafe {
            let b = arena::create_default_arena();
            if b.is_null() {
                return false;
            }

            state.free.insert(b);
            true
        }
    }

    unsafe fn alloc_large(needed_block: usize) -> *mut u8 {
        unsafe {
            let b = arena::create_large_arena(needed_block);
            if b.is_null() {
                return ptr::null_mut();
            }
            payload_ptr(b)
        }
    }

    unsafe fn split_and_take(
        state: &mut AllocState,
        b: *mut BlockHeader,
        needed: usize,
    ) -> *mut u8 {
        unsafe {
            let orig_size = (*b).size();
            let orig_flags = (*b).flags();
            let orig_first = (orig_flags & FIRST) != 0;
            let orig_last = (orig_flags & LAST) != 0;

            state.free.remove(b);

            let remainder = orig_size.saturating_sub(needed);
            if remainder >= min_free_block_size() {
                let r = (b as *mut u8).add(needed) as *mut BlockHeader;
                (*r).prev_size = needed;
                (*r).set_size_and_flags(remainder, if orig_last { LAST } else { 0 });

                // Fix "next block" prev_size if remainder is not last
                if !(*r).is_last() {
                    let after = next_block(r);
                    (*after).prev_size = remainder;
                }

                // Allocated block keeps FIRST, loses LAST (because we split)
                let mut a_flags = BUSY;
                if orig_first {
                    a_flags |= FIRST;
                }
                (*b).set_size_and_flags(needed, a_flags);

                state.free.insert(r);
            } else {
                // do not split; give an enire block
                let mut a_flags = BUSY;
                if orig_last {
                    a_flags |= LAST
                }
                if orig_first {
                    a_flags |= FIRST
                }
                (*b).set_size_and_flags(orig_size, a_flags);
            }

            payload_ptr(b)
        }
    }

    unsafe fn coalesce_and_insert(state: &mut AllocState, mut b: *mut BlockHeader) {
        unsafe {
            let flags_keep = (*b).flags() & (FIRST | LAST);
            let sz = (*b).size();
            (*b).set_size_and_flags(sz, flags_keep);

            // Coalesce with next
            if !(*b).is_last() {
                let n = next_block(b);
                if !(*n).is_busy() {
                    // Remove next from free list
                    state.free.remove(n);

                    let new_size = (*b).size() + (*n).size();
                    let new_flags = ((*b).flags() & FIRST) | ((*n).flags() & LAST);
                    (*b).set_size_and_flags(new_size, new_flags);

                    // Fix after-next prev_size if not last
                    if !(*b).is_last() {
                        let after = next_block(b);
                        (*after).prev_size = new_size;
                    }
                }
            }

            // Coalesce with prev
            if !(*b).is_first() {
                let p = prev_block(b);
                if !(*p).is_busy() {
                    // Remove prev from free list
                    state.free.remove(p);

                    let new_size = (*p).size() + (*b).size();
                    let new_flags = ((*p).flags() & FIRST) | ((*b).flags() & LAST);
                    (*p).set_size_and_flags(new_size, new_flags);

                    // Fix next prev_size if not last
                    if !(*p).is_last() {
                        let after = next_block(p);
                        (*after).prev_size = new_size;
                    }

                    b = p;
                }
            }

            state.free.insert(b);
        }
    }

    // pub fn debug_dump_state(&self, tag: &str) {
    //     let state = STATE.lock();
    //     unsafe { state.free.dump(tag) };
    // }
}

unsafe impl GlobalAlloc for ArenaAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            if layout.size() == 0 {
                return core::ptr::NonNull::<u8>::dangling().as_ptr();
            }

            // Keep it simple: require align <= 8 (or route to large path).
            let req_align = layout.align();
            if req_align > ALIGN {
                // Simple fallback: treat as "large"
                let needed = align_up(HEADER_SIZE + layout.size(), max(req_align, ALIGN));
                return Self::alloc_large(needed);
            }

            let needed = align_up(HEADER_SIZE + layout.size(), max(req_align, ALIGN));

            // Large allocation path if it cannot fit in a default arena.
            // (Default arenas are fixed size; large arenas are bigger than that.)
            if needed > default_arena_size() {
                return Self::alloc_large(needed);
            }

            let mut state = STATE.lock();

            // Ensure we have at least one arena
            if state.free.is_empty() {
                if !Self::add_default_arena(&mut state) {
                    return ptr::null_mut();
                }
            }

            // Find best fit; if none, add another arena and try again
            let mut b = state.free.find_best_fit(needed);
            if b.is_null() {
                if !Self::add_default_arena(&mut state) {
                    return ptr::null_mut();
                }
                b = state.free.find_best_fit(needed);
                if b.is_null() {
                    return ptr::null_mut();
                }
            }

            Self::split_and_take(&mut state, b, needed)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe {
            if ptr.is_null() {
                return;
            }

            let b = header_from_payload(ptr);
            let sz = (*b).size();

            // If it's bigger than the default arena size, it must have been a "large arena" mapping.
            if sz > default_arena_size() {
                os::unmap(b as *mut u8, sz);
                return;
            }

            let mut state = STATE.lock();
            Self::coalesce_and_insert(&mut state, b);
        }
    }
}
