use crate::block::{BlockHeader, free_node_ptr};

#[derive(Default)]
pub struct FreeList {
    pub head: *mut BlockHeader,
}

unsafe impl Send for FreeList {}
unsafe impl Sync for FreeList {}

impl FreeList {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    pub unsafe fn push_front(&mut self, b: *mut BlockHeader) {
        unsafe {
            let node = free_node_ptr(b);
            (*node).prev_free = core::ptr::null_mut();
            (*node).next_free = self.head;

            if !self.head.is_null() {
                let head_node = free_node_ptr(self.head);
                (*head_node).prev_free = b;
            }

            self.head = b;
        }
    }

    pub unsafe fn remove(&mut self, b: *mut BlockHeader) {
        unsafe {
            let node = free_node_ptr(b);
            let prev = (*node).prev_free;
            let next = (*node).next_free;

            if prev.is_null() {
                self.head = next;
            } else {
                let prev_node = free_node_ptr(prev);
                (*prev_node).next_free = next;
            }

            if !next.is_null() {
                let next_node = free_node_ptr(next);
                (*next_node).prev_free = prev;
            }

            (*node).next_free = core::ptr::null_mut();
            (*node).prev_free = core::ptr::null_mut();
        }
    }

    pub unsafe fn find_best_fit(&mut self, needed: usize) -> *mut BlockHeader {
        unsafe {
            let mut cur = self.head;
            let mut best: *mut BlockHeader = core::ptr::null_mut();
            let mut best_size = usize::MAX;

            while !cur.is_null() {
                let block_size = (*cur).size();
                if block_size >= needed && block_size < best_size {
                    best = cur;
                    best_size = block_size;

                    if block_size == needed {
                        break;
                    }
                }

                let cur_node = free_node_ptr(cur);
                cur = (*cur_node).next_free;
            }

            best
        }
    }

    pub unsafe fn dump(&self, tag: &str) {
        unsafe {
            eprintln!("--- FreeList dump: {tag} ---");
            let mut i = 0usize;
            let mut cur = self.head;

            while !cur.is_null() {
                let node = crate::block::free_node_ptr(cur);
                eprintln!(
                    "#{i}: block={:p} size={} flags={:#05b} prev_free={:p} next_free={:p}",
                    cur,
                    (*cur).size(),
                    (*cur).flags(),
                    (*node).prev_free,
                    (*node).next_free
                );

                cur = (*node).next_free;
                i += 1;

                if i > 10000 {
                    eprintln!("(stopping: list looks corrupted / cyclic)");
                    break;
                }
            }

            eprintln!("--- end dump ---");
        }
    }
}
