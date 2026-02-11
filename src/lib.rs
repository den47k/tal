mod allocator;
mod free;
mod heap;
mod os;
mod sync;

use allocator::ArenaAllocator;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PAGE_SIZE: usize = os::page_size();
}

pub static PAGE_ALLOCATOR: ArenaAllocator = ArenaAllocator;
