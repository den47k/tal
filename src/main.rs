use core::alloc::{GlobalAlloc, Layout};

fn main() {
    let page = *tal::PAGE_SIZE;
    println!("page size: {}", page);

    unsafe {
        let mut a_layout = Layout::from_size_align(1000, 8).unwrap();
        let mut b_layout = Layout::from_size_align(2000, 8).unwrap();

        let mut a = tal::PAGE_ALLOCATOR.alloc(a_layout);
        let mut b = tal::PAGE_ALLOCATOR.alloc(b_layout);
        assert!(!a.is_null() && !b.is_null());

        // realloc b to 4000
        let new_b = tal::PAGE_ALLOCATOR.realloc(b, b_layout, 4000);
        if new_b.is_null() {
            // realloc failed, b is still valid
            panic!("realloc(b) failed");
        }
        b = new_b;
        b_layout = Layout::from_size_align(4000, 8).unwrap();

        // realloc a to 40000
        let new_a = tal::PAGE_ALLOCATOR.realloc(a, a_layout, 40000);
        if new_a.is_null() {
            panic!("realloc(a) failed");
        }
        a = new_a;
        a_layout = Layout::from_size_align(40000, 8).unwrap();

        tal::PAGE_ALLOCATOR.dealloc(a, a_layout);
        tal::PAGE_ALLOCATOR.dealloc(b, b_layout);

        let a2 = tal::PAGE_ALLOCATOR.alloc(Layout::from_size_align(1000, 8).unwrap());
        tal::PAGE_ALLOCATOR.dealloc(a2, Layout::from_size_align(1000, 8).unwrap());
    }
}
