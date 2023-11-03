//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::{TaskControlBlock, current_process};
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self) -> isize;
    /// Unlock the mutex
    fn unlock(&self);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                // let task = current_task().unwrap();
                // let mut inner = task.inner_exclusive_access();
                // let mutex_id = inner.mutex_id;
                // inner.needed[mutex_id] += 1;
                // let process = current_process();
                // let process = process.inner_exclusive_access();
                // drop(inner);
                // if !process.check_deadlock() {
                //     return -0xDEAD;
                // }
                // drop(process);
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                // println!("get lock");
                // let task = current_task().unwrap();
                // let mut inner = task.inner_exclusive_access();
                // let mutex_id = inner.mutex_id;
                // inner.allocated[mutex_id] += 1;
                // if inner.needed[mutex_id] >= 1 {
                //     inner.needed[mutex_id] -= 1;
                // }
                // let process = current_process();
                // let mut process = process.inner_exclusive_access();
                // process.available[mutex_id] -= 1;
                // drop(inner);
                // drop(process);
                *locked = true;
                return 0;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        // let task = current_task().unwrap();
        // let mut inner = task.inner_exclusive_access();
        // let mutex_id = inner.mutex_id;
        // inner.allocated[mutex_id] -= 1;
        // let process = current_process();
        // let mut process = process.inner_exclusive_access();
        // process.available[mutex_id] += 1;
        // drop(inner);
        // drop(process);
        *locked = false;
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            let task = current_task().unwrap();
            let mut inner = task.inner_exclusive_access();
            let mutex_id = inner.mutex_id;
            inner.needed[mutex_id] += 1;
            drop(inner);
            let process = current_process();
            let process = process.inner_exclusive_access();
            if !process.check_deadlock() {
                return -0xdead;
            }
            drop(process);
            mutex_inner.wait_queue.push_back(task.clone());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            let task = current_task().unwrap();
            let mut inner = task.inner_exclusive_access();
            let mutex_id = inner.mutex_id;
            inner.allocated[mutex_id] += 1;
            let process = current_process();
            let mut process = process.inner_exclusive_access();
            process.available[mutex_id] -= 1;
            drop(inner);
            drop(process);
            mutex_inner.locked = true;
        }
        0
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        let task = current_task().unwrap();
        let mut inner = task.inner_exclusive_access();
        let mutex_id = inner.mutex_id;
        inner.allocated[mutex_id] -= 1;
        drop(inner);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            let task = waking_task.clone();
            let mut inner = task.inner_exclusive_access();
            inner.needed[mutex_id] -= 1;
            inner.allocated[mutex_id] += 1;
            drop(inner);
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
            let process = current_process();
            let mut process = process.inner_exclusive_access();
            process.available[mutex_id] += 1;
            drop(process);
        }
    }
}
