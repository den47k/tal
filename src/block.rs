pub const ALIGN: usize = 8;

pub const BUSY: usize = 0b001;
pub const FIRST: usize = 0b010;
pub const LAST: usize = 0b100;
pub const FLAG_MASK: usize = BUSY | FIRST | LAST;

#[repr(C)]
pub struct BlockHeader {
    pub size_and_flags: usize,
    pub prev_size: usize,
}

#[repr(C)]
pub struct FreeNode {
    pub prev_free: *mut BlockHeader,
    pub next_free: *mut BlockHeader,
}

pub const HEADER_SIZE: usize = size_of::<BlockHeader>();

#[inline]
pub const fn align_up(x: usize, a: usize) -> usize {
    (x + (a - 1)) & !(a - 1)
}

#[inline]
pub fn min_free_block_size() -> usize {
    align_up(HEADER_SIZE + size_of::<FreeNode>(), ALIGN)
}

impl BlockHeader {
    #[inline]
    pub fn size(&self) -> usize {
        self.size_and_flags & !FLAG_MASK
    }

    #[inline]
    pub fn flags(&self) -> usize {
        self.size_and_flags & FLAG_MASK
    }

    #[inline]
    pub fn is_busy(&self) -> bool {
        (self.flags() & BUSY) != 0
    }

    #[inline]
    pub fn is_first(&self) -> bool {
        (self.flags() & FIRST) != 0
    }

    #[inline]
    pub fn is_last(&self) -> bool {
        (self.flags() & LAST) != 0
    }

    #[inline]
    pub fn set_size_and_flags(&mut self, size: usize, flags: usize) {
        self.size_and_flags = size | (flags & FLAG_MASK)
    }
}

#[inline]
pub unsafe fn free_node_ptr(b: *mut BlockHeader) -> *mut FreeNode {
    unsafe { (b as *mut u8).add(HEADER_SIZE) as *mut FreeNode }
}

#[inline]
pub unsafe fn next_block(b: *mut BlockHeader) -> *mut BlockHeader {
    unsafe { (b as *mut u8).add((*b).size()) as *mut BlockHeader }
}

#[inline]
pub unsafe fn prev_block(b: *mut BlockHeader) -> *mut BlockHeader {
    unsafe { (b as *mut u8).sub((*b).prev_size) as *mut BlockHeader }
}

#[inline]
pub unsafe fn payload_ptr(b: *mut BlockHeader) -> *mut u8 {
    unsafe { (b as *mut u8).add(HEADER_SIZE) }
}

#[inline]
pub unsafe fn header_from_payload(p: *mut u8) -> *mut BlockHeader {
    unsafe { (p as *mut u8).sub(HEADER_SIZE) as *mut BlockHeader }
}
