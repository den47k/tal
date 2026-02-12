use core::ptr;

#[cfg(unix)]
pub fn page_size() -> usize {
    #[cfg(target_os = "linux")]
    unsafe {
        libc::sysconf(libc::_SC_PAGESIZE) as usize
    }
}

#[cfg(unix)]
pub fn map(len: usize) -> *mut u8 {
    unsafe {
        let addr = libc::mmap(
            ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if addr == libc::MAP_FAILED {
            return ptr::null_mut();
        } else {
            addr as *mut u8
        }
    }
}

#[cfg(unix)]
pub fn unmap(addr: *mut u8, len: usize) {
    unsafe {
        libc::munmap(addr as *mut libc::c_void, len);
    }
}

pub unsafe fn madvise(addr: *mut u8, len: usize) {
    unsafe {
        if addr.is_null() || len == 0 {
            return;
        }

        let _ = libc::madvise(addr as *mut libc::c_void, len, libc::MADV_DONTNEED);
    }
}
