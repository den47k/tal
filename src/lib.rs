mod allocator;
mod block;
mod free;
mod os;
mod spinlock;

use allocator::ArenaListAllocator;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PAGE_SIZE: usize = os::page_size();
}

/// Expose allocator instance like you already do.
pub static PAGE_ALLOCATOR: ArenaListAllocator = ArenaListAllocator;
