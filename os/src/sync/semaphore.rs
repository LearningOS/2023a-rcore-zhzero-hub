//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock, current_process};
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        let sem_id = task_inner.sem_id;
        task_inner.allocated_sem[sem_id] -= 1;
        drop(task_inner);
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                let task_clone = task.clone();
                let mut inner = task_clone.inner_exclusive_access();
                inner.needed_sem[sem_id] -= 1;
                inner.allocated_sem[sem_id] += 1;
                drop(inner);
                wakeup_task(task);
            }
        } else {
            let process = current_process();
            let mut process = process.inner_exclusive_access();
            process.available_sem[sem_id] += 1;
        }
    }

    /// down operation of semaphore
    pub fn down(&self) -> isize {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        let sem_id = task_inner.sem_id;
        if inner.count < 0 {
            task_inner.needed_sem[sem_id] += 1;
            drop(task_inner);
            let process = current_process();
            let process = process.inner_exclusive_access();
            if !process.check_deadlock_sem() {
                return -0xdead;
            }
            inner.wait_queue.push_back(current_task().unwrap());
            drop(process);
            drop(inner);
            block_current_and_run_next();
        } else {
            let process = current_process();
            let mut process = process.inner_exclusive_access();
            task_inner.allocated_sem[sem_id] += 1;
            process.available_sem[sem_id] -= 1;
            drop(process);
        }
        0
    }
}
