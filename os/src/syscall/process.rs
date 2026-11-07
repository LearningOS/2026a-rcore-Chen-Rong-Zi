//! Process management syscalls
use crate::mm::{translated_byte_buffer, MapPermission, PTEFlags, PageTable, VirtAddr};
use crate::task::{
    change_program_brk, current_memory_set, current_user_token, exit_current_and_run_next,
    get_syscall_count, suspend_current_and_run_next,
};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let token = current_user_token();
    let buffers = translated_byte_buffer(
        token,
        ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );
    let time_val_bytes = unsafe {
        core::slice::from_raw_parts(
            &time_val as *const _ as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    let mut offset = 0;
    for buffer in buffers {
        let len = buffer.len();
        buffer.copy_from_slice(&time_val_bytes[offset..offset + len]);
        offset += len;
    }
    0
}

/// Trace request: read/write a user byte or query syscall count.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            // Read a byte from user address `id`.
            let addr = id;
            let page_table = PageTable::from_token(current_user_token());
            let vpn = VirtAddr(addr).floor();
            if let Some(pte) = page_table.translate(vpn) {
                if pte.is_valid()
                    && pte.readable()
                    && pte.flags().contains(PTEFlags::U)
                {
                    let byte = pte.ppn().get_bytes_array()[VirtAddr(addr).page_offset()];
                    return byte as isize;
                }
            }
            -1
        }
        1 => {
            // Write a byte to user address `id`.
            let addr = id;
            let page_table = PageTable::from_token(current_user_token());
            let vpn = VirtAddr(addr).floor();
            if let Some(pte) = page_table.translate(vpn) {
                if pte.is_valid()
                    && pte.writable()
                    && pte.flags().contains(PTEFlags::U)
                {
                    let byte = &mut pte.ppn().get_bytes_array()[VirtAddr(addr).page_offset()];
                    *byte = (data & 0xff) as u8;
                    return 0;
                }
            }
            -1
        }
        2 => get_syscall_count(id) as isize,
        _ => panic!("Unsupported trace_request: {}", trace_request),
    }
}

/// Map a range of anonymous memory into the current address space.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }
    if prot & !0x7 != 0 {
        return -1;
    }
    if prot & 0x7 == 0 {
        return -1;
    }
    let end_va: VirtAddr = VirtAddr(start + len).ceil().into();
    let memory_set = current_memory_set();
    if memory_set.has_any_mapped(start_va, end_va) {
        return -1;
    }
    let mut map_perm = MapPermission::U;
    if prot & 0x1 != 0 {
        map_perm |= MapPermission::R;
    }
    if prot & 0x2 != 0 {
        map_perm |= MapPermission::W;
    }
    if prot & 0x4 != 0 {
        map_perm |= MapPermission::X;
    }
    memory_set.insert_framed_area(start_va, end_va, map_perm);
    0
}

/// Unmap a range of memory from the current address space.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }
    let end_va: VirtAddr = VirtAddr(start + len).ceil().into();
    let memory_set = current_memory_set();
    if !memory_set.is_mapped_range(start_va, end_va) {
        return -1;
    }
    memory_set.unmap_range(start_va, end_va);
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
