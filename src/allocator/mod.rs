use core::alloc::{GlobalAlloc, Layout};
use core::cmp::{max, min};
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

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe {
            if ptr.is_null() {
                let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
                    return core::ptr::null_mut();
                };
                return self.alloc(new_layout);
            }

            if new_size == 0 {
                self.dealloc(ptr, layout);
                return core::ptr::null_mut();
            }

            let b = header_from_payload(ptr);
            let old_total = (*b).size();
            let old_payload = old_total.saturating_sub(HEADER_SIZE);

            let req_align = layout.align();
            let needed_total = align_up(HEADER_SIZE + new_size, max(req_align, ALIGN));

            // Large arena reallocation
            if old_total > default_arena_size() {
                if needed_total <= old_total {
                    return ptr;
                }

                let new_ptr = Self::alloc_large(needed_total);
                if new_ptr.is_null() {
                    return core::ptr::null_mut();
                }

                core::ptr::copy_nonoverlapping(ptr, new_ptr, min(old_payload, new_size));
                arena::destroy_arena(b as *mut u8, old_total);
            }

            // Default arena reallocation
            // Case A: current block already large enough -> optionally shrink by splitting tail.
            if needed_total < old_total {
                let remainder = old_total - needed_total;

                if remainder >= min_free_block_size() {
                    let mut state = STATE.lock();

                    let orig_flags = (*b).flags();
                    let orig_first = (orig_flags & FIRST) != 0;
                    let orig_last = (orig_flags & LAST) != 0;

                    let mut a_flags = BUSY;
                    if orig_first {
                        a_flags |= FIRST;
                    }
                    (*b).set_size_and_flags(needed_total, a_flags);

                    let r = (b as *mut u8).add(needed_total) as *mut BlockHeader;
                    (*r).prev_size = needed_total;
                    (*r).set_size_and_flags(remainder, if orig_last { LAST } else { 0 });

                    if !(*r).is_last() {
                        (*next_block(r)).prev_size = remainder;
                    }

                    Self::coalesce_and_insert(&mut state, r);
                }
                return ptr;
            }

            // Case B: need to grow. Try to merge with next free block
            {
                let mut state = STATE.lock();

                if !(*b).is_last() {
                    let n = next_block(b);

                    if !(*n).is_busy() {
                        let combined = old_total + (*n).size();

                        if combined >= needed_total {
                            state.free.remove(n);

                            let orig_first = ((*b).flags() & FIRST) != 0;
                            let merged_last = ((*n).flags() & LAST) != 0;

                            let extra = combined - needed_total;

                            if extra >= min_free_block_size() {
                                let mut a_flags = BUSY;
                                if orig_first {
                                    a_flags |= FIRST;
                                }
                                (*b).set_size_and_flags(needed_total, a_flags);

                                let r = (b as *mut u8).add(needed_total) as *mut BlockHeader;
                                (*r).prev_size = needed_total;
                                (*r).set_size_and_flags(extra, if merged_last { LAST } else { 0 });

                                if !(*r).is_last() {
                                    (*next_block(r)).prev_size = extra;
                                }

                                Self::coalesce_and_insert(&mut state, r);
                            } else {
                                let mut a_flags = BUSY;
                                if orig_first {
                                    a_flags |= FIRST;
                                }
                                if merged_last {
                                    a_flags |= LAST;
                                }

                                (*b).set_size_and_flags(combined, a_flags);

                                if !merged_last {
                                    (*next_block(b)).prev_size = combined;
                                }
                            }

                            return ptr;
                        }
                    }
                }
            }

            // Case C: cannot grow in place -> allocate new, copy, free old.
            let Ok(new_layout) = Layout::from_size_align(new_size, req_align) else {
                return core::ptr::null_mut();
            };

            let new_ptr = self.alloc(new_layout);
            if new_ptr.is_null() {
                return core::ptr::null_mut();
            }

            core::ptr::copy_nonoverlapping(ptr, new_ptr, min(old_payload, new_size));
            self.dealloc(ptr, layout);
            new_ptr
        }
    }
}
