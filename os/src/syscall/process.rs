//! Process management syscalls
use core::usize;

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, current_user_token, get_current_task, TaskControlBlock,
    }, timer::{get_time_us}, mm::{translated_byte_buffer, VirtAddr},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        let sec_addr = (&(*_ts).sec) as *const usize;
        let addr = translated_byte_buffer(current_user_token(), sec_addr as *const u8, usize::BITS as usize);
        *(addr[0].as_ptr() as *mut usize) = us / 1_000_000;
        let usec_addr = (&(*_ts).usec) as *const usize;
        let addr = translated_byte_buffer(current_user_token(), usec_addr as *const u8, usize::BITS as usize);
        *(addr[0].as_ptr() as *mut usize) = us % 1_000_000;
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let info = get_current_task();
    unsafe {
        let syscall_times_addr = &((*_ti).syscall_times) as *const u32;
        let addr = translated_byte_buffer(current_user_token(), syscall_times_addr as *const u8, u32::BITS as usize * MAX_SYSCALL_NUM);
        (addr[0].as_ptr() as *mut u32).copy_from((*info).run_time.as_ptr(), MAX_SYSCALL_NUM);
        let time_addr = &((*_ti).time) as *const usize;
        let addr = translated_byte_buffer(current_user_token(), time_addr as *const u8, usize::BITS as usize);
        // println!("get time: {}, start time: {}", get_time_us() / 1_000_000, (*info).start_time);
        let time_ms: usize = get_time_us() / 1_000;
        let time = time_ms - (*info).start_time;
        println!("time: {}, time_ms: {}, start_time: {}", time, time_ms, (*info).start_time);
        *(addr[0].as_ptr() as *mut usize) = time;
        let status_addr = &((*_ti).status) as *const TaskStatus;
        let addr = translated_byte_buffer(current_user_token(), status_addr as *const u8, 1);
        *(addr[0].as_ptr() as *mut TaskStatus) = (*info).task_status;
        println!("syscall_times_addr: {:#x}, time_addr: {:#x}, status_addr: {:#x}", syscall_times_addr as usize, time_addr as usize, status_addr as usize);
    }
    trace!("kernel: sys_task_info finish!");
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    println!("sys_mmap start: {:#x}, len: {:#x}, port: {:#x}", _start, _len, _port);
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if (_start % PAGE_SIZE != 0) || (_port & !0x7 != 0) || (_port & 0x7 == 0) {
        return -1;
    }
    let m = _len % PAGE_SIZE;
    let mut _len = _len;
    if m != 0 {
        _len += PAGE_SIZE - m;
    }
    let task = get_current_task() as *mut TaskControlBlock;
    unsafe {
        if (*task).memory_set.check_unused(VirtAddr(_start), VirtAddr(_start+_len)) {
            (*task).memory_set.add_virtual(VirtAddr(_start), VirtAddr(_start+_len), _port);
        } else {
            return -1;
        }
    }
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    let m = _len % PAGE_SIZE;
    let mut _len = _len;
    if m != 0 {
        _len += PAGE_SIZE - m;
    }
    let task = get_current_task() as *mut TaskControlBlock;
    let mut tmp = _start;
    unsafe {
        while tmp < _start+_len {
            if (*task).memory_set.check_unused(VirtAddr(tmp), VirtAddr(tmp+PAGE_SIZE)) {
                return -1;
            }
            tmp += PAGE_SIZE;
        }
        (*task).memory_set.remove_virtual(VirtAddr(_start), VirtAddr(_start+_len))
    }
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
