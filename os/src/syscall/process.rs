//! Process management syscalls
//!
use alloc::sync::Arc;

use crate::{
    fs::{open_file, OpenFlags},
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus, new_process, PROCESSOR
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
    syscall_times: [usize; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        println!("exit code {}", exit_code);
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
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
    let bind = PROCESSOR.exclusive_access().current().unwrap();
    let info = bind.inner_exclusive_access();
    unsafe {
        let syscall_times_addr = &((*_ti).syscall_times) as *const usize;
        let addr = translated_byte_buffer(current_user_token(), syscall_times_addr as *const u8, usize::BITS as usize * MAX_SYSCALL_NUM);
        (addr[0].as_ptr() as *mut usize).copy_from((*info).run_time.as_ptr(), MAX_SYSCALL_NUM);
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

/// YOUR JOB: Implement mmap.
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
    let bind = PROCESSOR.exclusive_access().current().unwrap();
    let mut task = bind.inner_exclusive_access();
    if task.memory_set.check_unused(VirtAddr(_start), VirtAddr(_start+_len)) {
        task.memory_set.add_virtual(VirtAddr(_start), VirtAddr(_start+_len), _port);
    } else {
        return -1;
    }
    0
}

/// YOUR JOB: Implement munmap.
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
    let bind = PROCESSOR.exclusive_access().current().unwrap();
    let mut task = bind.inner_exclusive_access();
    let mut tmp = _start;
    while tmp < _start+_len {
        if task.memory_set.check_unused(VirtAddr(tmp), VirtAddr(tmp+PAGE_SIZE)) {
            return -1;
        }
        tmp += PAGE_SIZE;
    }
    task.memory_set.remove_virtual(VirtAddr(_start), VirtAddr(_start+_len));
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    println!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    new_process(_path)
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _prio >= 2 {
        let bind = PROCESSOR.exclusive_access().current().unwrap();
        let mut task = bind.inner_exclusive_access();
        task.priori = _prio;
        _prio
    } else {
        -1
    }
}
