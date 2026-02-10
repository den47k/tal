use core::alloc::{GlobalAlloc, Layout};

fn main() {
    let page = *test2_alloc::PAGE_SIZE;
    println!("page size: {}", page);

    unsafe {
        test2_alloc::PAGE_ALLOCATOR.debug_dump_state("start");
        // Small allocs (should come from default arenas)
        let a_layout = Layout::from_size_align(1000, 8).unwrap();
        let b_layout = Layout::from_size_align(2000, 8).unwrap();
        let c_layout = Layout::from_size_align(8000, 8).unwrap();
        let d_layout = Layout::from_size_align(22000, 8).unwrap();

        let a = test2_alloc::PAGE_ALLOCATOR.alloc(a_layout);

        test2_alloc::PAGE_ALLOCATOR.debug_dump_state("after first alloc");

        let b = test2_alloc::PAGE_ALLOCATOR.alloc(b_layout);

        test2_alloc::PAGE_ALLOCATOR.debug_dump_state("after second alloc");

        let c = test2_alloc::PAGE_ALLOCATOR.alloc(c_layout);

        test2_alloc::PAGE_ALLOCATOR.debug_dump_state("after allocs");

        println!("a = {:p}, b = {:p}, c = {:p}", a, b, c);

        assert!(!a.is_null() && !b.is_null() && !c.is_null());

        // Touch memory to ensure mapping is usable
        *a = 0xAA;
        *a.add(999) = 0xAB;

        *b = 0xBA;
        *b.add(1999) = 0xBB;

        *c = 0xCA;
        *c.add(2999) = 0xCB;

        // Free middle first, then ends -> exercises coalescing cases
        test2_alloc::PAGE_ALLOCATOR.dealloc(b, b_layout);

        test2_alloc::PAGE_ALLOCATOR.debug_dump_state("after dealloc b");

        test2_alloc::PAGE_ALLOCATOR.alloc(d_layout);

        test2_alloc::PAGE_ALLOCATOR.debug_dump_state("after alloc b again");

        test2_alloc::PAGE_ALLOCATOR.dealloc(a, a_layout);
        test2_alloc::PAGE_ALLOCATOR.dealloc(c, c_layout);

        println!("freed a,b,c (should have coalesced back in arena)");
    }
}
