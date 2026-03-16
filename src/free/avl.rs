use core::ptr;

use crate::heap::block::{BlockHeader, links_ptr};

#[derive(Default)]
pub struct FreeTree {
    root: *mut BlockHeader,
}

unsafe impl Send for FreeTree {}

impl FreeTree {
    pub const fn new() -> Self {
        Self {
            root: ptr::null_mut(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.root.is_null()
    }

    pub unsafe fn find_best_fit(&self, needed: usize) -> *mut BlockHeader {
        unsafe {
            let mut cur = self.root;
            let mut best: *mut BlockHeader = ptr::null_mut();

            while !cur.is_null() {
                let sz = (*cur).size();
                if sz >= needed {
                    best = cur;
                    cur = (*links_ptr(cur)).left;
                } else {
                    cur = (*links_ptr(cur)).right;
                }
            }

            best
        }
    }

    pub unsafe fn insert(&mut self, b: *mut BlockHeader) {
        unsafe {
            init_free_links(b);

            let mut cur = self.root;
            let mut parent: *mut BlockHeader = ptr::null_mut();

            while !cur.is_null() {
                parent = cur;
                let bs = (*b).size();
                let cs = (*cur).size();
                if bs > cs {
                    cur = (*links_ptr(cur)).right;
                } else if bs < cs {
                    cur = (*links_ptr(cur)).left;
                } else {
                    list_push_front(cur, b);
                    return;
                }
            }

            (*links_ptr(b)).parent = parent;

            if parent.is_null() {
                self.root = b;
            } else {
                let ps = (*parent).size();
                let bs = (*b).size();
                if bs < ps {
                    (*links_ptr(parent)).left = b;
                } else {
                    (*links_ptr(parent)).right = b;
                }
            }

            rebalance_upwards(self, parent);
        }
    }

    pub unsafe fn remove(&mut self, b: *mut BlockHeader) {
        unsafe {
            let head = self.find_head_by_size((*b).size());
            if head.is_null() {
                return;
            }

            if head != b {
                list_remove(b);
                return;
            }

            let promoted = (*links_ptr(b)).same_next;
            if !promoted.is_null() {
                list_remove(promoted);
                promote_replace_node(self, b, promoted);
                init_free_links(b);
                return;
            }

            avl_delete_node(self, b);
        }
    }

    unsafe fn find_head_by_size(&self, size: usize) -> *mut BlockHeader {
        unsafe {
            let mut cur = self.root;
            while !cur.is_null() {
                let cs = (*cur).size();
                if size < cs {
                    cur = (*links_ptr(cur)).left;
                } else if size > cs {
                    cur = (*links_ptr(cur)).right;
                } else {
                    return cur;
                }
            }
            ptr::null_mut()
        }
    }
}

unsafe fn init_free_links(b: *mut BlockHeader) {
    unsafe {
        let l = links_ptr(b);
        (*l).same_prev = ptr::null_mut();
        (*l).same_next = ptr::null_mut();
        (*l).left = ptr::null_mut();
        (*l).right = ptr::null_mut();
        (*l).parent = ptr::null_mut();
        (*l).height = 1;
    }
}

// head of the "same-size" list is an avl node and b is inserted after the head.
// H -> A <-> B <-> C
unsafe fn list_push_front(head: *mut BlockHeader, b: *mut BlockHeader) {
    unsafe {
        let hl = links_ptr(head);
        let bl = links_ptr(b);

        (*bl).same_prev = head;
        (*bl).same_next = (*hl).same_next;

        if !(*hl).same_next.is_null() {
            (*links_ptr((*hl).same_next)).same_prev = b;
        }

        (*hl).same_next = b;
        (*bl).parent = ptr::null_mut();
        (*bl).left = ptr::null_mut();
        (*bl).right = ptr::null_mut();
        (*bl).height = 1;
    }
}

unsafe fn list_remove(b: *mut BlockHeader) {
    unsafe {
        let bl = links_ptr(b);
        let p = (*bl).same_prev;
        let n = (*bl).same_next;

        if !p.is_null() {
            (*links_ptr(p)).same_next = n;
        }
        if !n.is_null() {
            (*links_ptr(n)).same_prev = p;
        }

        (*bl).same_prev = ptr::null_mut();
        (*bl).same_next = ptr::null_mut();
    }
}

// promote `new_head` to replace `old_head` in the AVL tree (same size).
unsafe fn promote_replace_node(
    tree: &mut FreeTree,
    old_head: *mut BlockHeader,
    new_head: *mut BlockHeader,
) {
    unsafe {
        // copy AVL fields from old to new
        let old = links_ptr(old_head);
        let new = links_ptr(new_head);

        (*new).left = (*old).left;
        (*new).right = (*old).right;
        (*new).parent = (*old).parent;
        (*new).height = (*old).height;

        // adopt same-size list of old head:
        (*new).same_prev = ptr::null_mut();
        (*new).same_next = (*old).same_next;
        if !(*old).same_next.is_null() {
            (*links_ptr((*old).same_next)).same_prev = new_head;
        }

        // fix children parent pointers to new head
        if !(*new).left.is_null() {
            (*links_ptr((*new).left)).parent = new_head;
        }
        if !(*new).right.is_null() {
            (*links_ptr((*new).right)).parent = new_head;
        }

        // fix parent link to point to new head
        if (*new).parent.is_null() {
            tree.root = new_head;
        } else {
            let p = (*new).parent;
            if (*links_ptr(p)).left == old_head {
                (*links_ptr(p)).left = new_head;
            } else if (*links_ptr(p)).right == old_head {
                (*links_ptr(p)).right = new_head;
            }
        }

        // detach old head (no longer in tree/list)
        (*old).left = ptr::null_mut();
        (*old).right = ptr::null_mut();
        (*old).parent = ptr::null_mut();
        (*old).height = 1;
        (*old).same_prev = ptr::null_mut();
        (*old).same_next = ptr::null_mut();
    }
}

// helpers
unsafe fn h(b: *mut BlockHeader) -> i32 {
    unsafe {
        if b.is_null() {
            0
        } else {
            (*links_ptr(b)).height
        }
    }
}

unsafe fn update_height(b: *mut BlockHeader) {
    unsafe {
        let l = (*links_ptr(b)).left;
        let r = (*links_ptr(b)).right;
        (*links_ptr(b)).height = 1 + core::cmp::max(h(l), h(r));
    }
}

unsafe fn balance_factor(b: *mut BlockHeader) -> i32 {
    unsafe { h((*links_ptr(b)).left) - h((*links_ptr(b)).right) }
}

unsafe fn rotate_left(tree: &mut FreeTree, x: *mut BlockHeader) -> *mut BlockHeader {
    unsafe {
        let y = (*links_ptr(x)).right;
        let t2 = (*links_ptr(y)).left;

        (*links_ptr(y)).left = x;
        (*links_ptr(x)).right = t2;

        //parents
        let xp = (*links_ptr(x)).parent;
        (*links_ptr(y)).parent = xp;
        (*links_ptr(x)).parent = y;
        if !t2.is_null() {
            (*links_ptr(t2)).parent = x;
        }

        if xp.is_null() {
            tree.root = y;
        } else if (*links_ptr(xp)).left == x {
            (*links_ptr(xp)).left = y;
        } else {
            (*links_ptr(xp)).right = y;
        }

        update_height(x);
        update_height(y);
        y
    }
}

unsafe fn rotate_right(tree: &mut FreeTree, y: *mut BlockHeader) -> *mut BlockHeader {
    unsafe {
        let x = (*links_ptr(y)).left;
        let t2 = (*links_ptr(x)).right;

        (*links_ptr(x)).right = y;
        (*links_ptr(y)).left = t2;

        // parents
        let yp = (*links_ptr(y)).parent;
        (*links_ptr(x)).parent = yp;
        (*links_ptr(y)).parent = x;
        if !t2.is_null() {
            (*links_ptr(t2)).parent = y;
        }

        if yp.is_null() {
            tree.root = x;
        } else if (*links_ptr(yp)).left == y {
            (*links_ptr(yp)).left = x;
        } else {
            (*links_ptr(yp)).right = x;
        }

        update_height(y);
        update_height(x);
        x
    }
}

unsafe fn rebalance_node(tree: &mut FreeTree, n: *mut BlockHeader) {
    unsafe {
        update_height(n);
        let bf = balance_factor(n);

        if bf > 1 {
            let l = (*links_ptr(n)).left;
            if balance_factor(l) < 0 {
                rotate_left(tree, l);
            }
            rotate_right(tree, n);
        } else if bf < -1 {
            let r = (*links_ptr(n)).right;
            if balance_factor(r) > 0 {
                rotate_right(tree, r);
            }
            rotate_left(tree, n);
        }
    }
}

unsafe fn rebalance_upwards(tree: &mut FreeTree, mut cur: *mut BlockHeader) {
    unsafe {
        while !cur.is_null() {
            rebalance_node(tree, cur);
            cur = (*links_ptr(cur)).parent;
        }
    }
}

unsafe fn avl_delete_node(tree: &mut FreeTree, z: *mut BlockHeader) {
    unsafe {
        let rebalance_from: *mut BlockHeader;

        if (*links_ptr(z)).left.is_null() || (*links_ptr(z)).right.is_null() {
            let child = if !(*links_ptr(z)).left.is_null() {
                (*links_ptr(z)).left
            } else {
                (*links_ptr(z)).right
            };

            let p = (*links_ptr(z)).parent;
            rebalance_from = p;

            if p.is_null() {
                tree.root = child;
            } else if (*links_ptr(p)).left == z {
                (*links_ptr(p)).left = child;
            } else {
                (*links_ptr(p)).right = child;
            }

            if !child.is_null() {
                (*links_ptr(child)).parent = p;
            }

            init_free_links(z);
        } else {
            let mut s = (*links_ptr(z)).right;
            while !(*links_ptr(s)).left.is_null() {
                s = (*links_ptr(s)).left;
            }

            // Detach successor from its current place
            let sp = (*links_ptr(s)).parent;
            let sr = (*links_ptr(s)).right;

            if (*links_ptr(sp)).left == s {
                (*links_ptr(sp)).left = sr;
            } else {
                (*links_ptr(sp)).right = sr;
            }
            if !sr.is_null() {
                (*links_ptr(sr)).parent = sp;
            }

            // Transplant s into z's position
            let zp = (*links_ptr(z)).parent;
            let zl = (*links_ptr(z)).left;
            let zr = (*links_ptr(z)).right;

            (*links_ptr(s)).parent = zp;
            (*links_ptr(s)).left = zl;
            (*links_ptr(s)).right = zr;
            (*links_ptr(s)).height = (*links_ptr(z)).height;

            if !zl.is_null() {
                (*links_ptr(zl)).parent = s;
            }
            if !zr.is_null() {
                (*links_ptr(zr)).parent = s;
            }

            if zp.is_null() {
                tree.root = s;
            } else if (*links_ptr(zp)).left == z {
                (*links_ptr(zp)).left = s;
            } else {
                (*links_ptr(zp)).right = s;
            }

            // detach z completely
            init_free_links(z);

            rebalance_from = if sp == z { s } else { sp };
        }

        rebalance_upwards(tree, rebalance_from);
    }
}
