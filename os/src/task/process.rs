//! Implementation of  [`ProcessControlBlock`]

use super::id::RecycleAllocator;
use super::manager::insert_into_pid2process;
use super::TaskControlBlock;
use super::{add_task, SignalFlags};
use super::{pid_alloc, PidHandle};
use crate::fs::{File, Stdin, Stdout};
use crate::mm::{MemorySet, KERNEL_SPACE};
use crate::sync::{Condvar, Mutex, Semaphore, UPSafeCell};
use crate::trap::{trap_handler, TrapContext};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefMut;
use crate::loaders::ElfLoader;

/// Process Control Block
pub struct ProcessControlBlock {
    /// immutable
    pub pid: PidHandle,
    /// mutable
    inner: UPSafeCell<ProcessControlBlockInner>,
}

/// Inner of Process Control Block
pub struct ProcessControlBlockInner {
    /// is zombie?
    pub is_zombie: bool,
    /// memory set(address space)
    pub memory_set: MemorySet,
    /// parent process
    pub parent: Option<Weak<ProcessControlBlock>>,
    /// children process
    pub children: Vec<Arc<ProcessControlBlock>>,
    /// exit code
    pub exit_code: i32,
    /// file descriptor table
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    /// signal flags
    pub signals: SignalFlags,
    /// tasks(also known as threads)
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    /// task resource allocator
    pub task_res_allocator: RecycleAllocator,
    /// mutex list
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    /// semaphore list
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    /// condvar list
    pub condvar_list: Vec<Option<Arc<Condvar>>>,
    /// deadlock flag
    pub deadlock_flag: bool,
    /// available
    pub available: [usize; 100],
    /// available
    pub available_sem: [usize; 100],
}

impl ProcessControlBlockInner {
    pub fn check_deadlock_sem(&self) -> bool {
        let flag = self.deadlock_flag;
        if flag {
            println!("start check deadlock sem");
            let mut check: [bool; 100] = [false; 100]; // 下标是任务
            let mut work: [usize; 100] = [0; 100];  // 下标是mutex_id
            let n = self.tasks.len();
            // print!("available: ");
            for (i, v) in self.available_sem.iter().enumerate() {
                // print!("{} ", *v);
                work[i] = *v;
            }
            // println!("");
            loop {
                let mut modified = false;
                println!("\x1b[32m checked: {:?} \x1b[0m", check);
                for (i, task) in self.tasks.iter().enumerate() {
                    if !check[i] {
                        if let Some(task) = task {
                            let mut check_flag = true;
                            let inner = task.inner_exclusive_access();
                            for (sem_id, value) in inner.needed_sem.iter().enumerate() {
                                // if sem_id < 4 {
                                //     print!("\x1b[32m sem_id: {}, need: {}, allocated: {} \x1b[0m", sem_id, *value, inner.allocated_sem[sem_id]);
                                // }
                                if *value > work[sem_id] {
                                    check_flag = false;
                                }
                            }
                            // println!("");
                            check[i] = check_flag;
                            if check_flag {
                                modified = true;
                                for sem_id in 0..100 {
                                    work[sem_id] += inner.allocated_sem[sem_id];
                                }
                            }
                        }
                    }
                }
                if !modified {
                    break;
                }
            }
            for i in 0..n {
                if !check[i] {
                    println!("deadlocked");
                    return false;
                }
            }
            println!("no deadlocked");
            true
        } else {
            true
        }
    }
    pub fn check_deadlock(&self) -> bool {
        let flag = self.deadlock_flag;
        if flag {
            println!("start check deadlock lock");
            let mut check: [bool; 100] = [false; 100]; // 下标是任务
            let mut work: [usize; 100] = [0; 100];  // 下标是mutex_id
            let n = self.tasks.len();
            print!("available: ");
            for (i, v) in self.available.iter().enumerate() {
                if i < 10 {
                    print!("{} ", *v);
                }
                work[i] = *v;
            }
            println!("");
            loop {
                let mut modified = false;
                println!("\x1b[32m checked: {:?} \x1b[0m", check);
                for (i, task) in self.tasks.iter().enumerate() {
                    if !check[i] {
                        if let Some(task) = task {
                            let mut check_flag = true;
                            let inner = task.inner_exclusive_access();
                            for (mutex_id, value) in inner.needed.iter().enumerate() {
                                if mutex_id < 4 {
                                    print!("\x1b[32m mutex_id: {}, need: {}, allocated: {} \x1b[0m", mutex_id, *value, inner.allocated[mutex_id]);
                                }
                                if *value > work[mutex_id] {
                                    check_flag = false;
                                }
                            }
                            println!("");
                            check[i] = check_flag;
                            if check_flag {
                                modified = true;
                                for mutex_id in 0..100 {
                                    work[mutex_id] += inner.allocated[mutex_id];
                                }
                            }
                        }
                    }
                }
                if !modified {
                    break;
                }
            }
            for i in 0..n {
                if !check[i] {
                    println!("deadlocked");
                    return false;
                }
            }
            println!("no deadlocked");
            true
        } else {
            true
        }
    }
    /// set deadlock
    pub fn set_deadlock_flag(&mut self, flag: bool) {
        self.deadlock_flag = flag;
    }
    #[allow(unused)]
    /// get the address of app's page table
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// allocate a new file descriptor
    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
    /// allocate a new task id
    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }
    /// deallocate a task id
    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }
    /// the count of tasks(threads) in this process
    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }
    /// get a task with tid in this process
    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }
}

impl ProcessControlBlock {
    /// set deadlock flag
    pub fn set_deadlock_flag(&self, flag: bool) {
        self.inner.exclusive_access().set_deadlock_flag(flag);
    }
    /// inner_exclusive_access
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }
    /// new process from elf file
    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        trace!("kernel: ProcessControlBlock::new");
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        // allocate a pid
        let pid_handle = pid_alloc();
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: vec![
                        // 0 -> stdin
                        Some(Arc::new(Stdin)),
                        // 1 -> stdout
                        Some(Arc::new(Stdout)),
                        // 2 -> stderr
                        Some(Arc::new(Stdout)),
                    ],
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    deadlock_flag: false,
                    available: [0; 100],
                    available_sem: [0; 100],
                })
            },
        });
        // create a main thread, we should allocate ustack and trap_cx here
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            ustack_base,
            true,
        ));
        // prepare trap_cx of main thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            ustack_top,
            KERNEL_SPACE.exclusive_access().token(),
            kstack_top,
            trap_handler as usize,
        );
        // add main thread to the process
        let mut process_inner = process.inner_exclusive_access();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }

    /// Only support processes with a single thread.
    pub fn exec(self: &Arc<Self>, elf_data: &[u8], _args: Vec<String>) {
        trace!("kernel: exec");
        assert_eq!(self.inner_exclusive_access().thread_count(), 1);
        // memory_set with elf program headers/trampoline/trap context/user stack
        trace!("kernel: exec .. MemorySet::from_elf");
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        let new_token = memory_set.token();
        // substitute memory_set
        trace!("kernel: exec .. substitute memory_set");
        self.inner_exclusive_access().memory_set = memory_set;
        // then we alloc user resource for main thread again
        // since memory_set has been changed
        trace!("kernel: exec .. alloc user resource for main thread again");
        let task = self.inner_exclusive_access().get_task(0);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.res.as_mut().unwrap().ustack_base = ustack_base;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();
        // push arguments on user stack
        trace!("kernel: exec .. push arguments on user stack");
        let mut user_sp = task_inner.res.as_mut().unwrap().ustack_top();
        let mut args = Vec::new();
        args.push(String::from("hello"));
        let loader: ElfLoader = ElfLoader::new(elf_data).unwrap();
        let len = args.len();
        user_sp = loader.init_stack(new_token, user_sp, args);
        // println!("0");
        // let mut args = Vec::new();
        // // args.push(String::from_utf8(elf_data.to_vec()).unwrap());
        // args.push(String::from("hello"));
        // let argc_size = core::mem::size_of::<usize>();
        // let argv_addr_size = args.len() * core::mem::size_of::<usize>();
        // let argv_data_size = args.iter().fold(0, |sum, s| sum + s.len() + 1);
        // user_sp -= argc_size;
        // // let argc_addr = user_sp;
        // user_sp -= argv_addr_size;
        // // let argv_addr_addr = user_sp;
        // user_sp -= argv_data_size;
        // user_sp -= user_sp % core::mem::size_of::<usize>();
        // // let argv_data_addr = user_sp;
        
        // *translated_refmut(new_token, user_sp as *mut usize) = args.len();
        // let argv_base = user_sp + argc_size;
        // let mut argv: Vec<_> = (0..args.len())
        // .map(|arg| {
        //     translated_refmut(
        //         new_token,
        //         (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
        //     )
        // })
        // .collect();
        
        // let mut argv_data_addr = user_sp + argc_size + argv_addr_size;
        // for i in 0..args.len() {
        //     *argv[i] = argv_data_addr;
        //     let mut p = argv_data_addr;
        //     for c in args[i].as_bytes() {
        //         *translated_refmut(new_token, p as *mut u8) = *c;
        //         p += 1;
        //     }
        //     *translated_refmut(new_token, p as *mut u8) = 0;
        //     argv_data_addr += args[i].len() + 1;
        // }

        // user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        // let argv_base = user_sp;
        // let mut argv: Vec<_> = (0..=args.len())
        //     .map(|arg| {
        //         translated_refmut(
        //             new_token,
        //             (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
        //         )
        //     })
        //     .collect();
        // *argv[args.len()] = 0;
        // for i in 0..args.len() {
        //     user_sp -= args[i].len() + 1;
        //     *argv[i] = user_sp;
        //     let mut p = user_sp;
        //     for c in args[i].as_bytes() {
        //         *translated_refmut(new_token, p as *mut u8) = *c;
        //         p += 1;
        //     }
        //     *translated_refmut(new_token, p as *mut u8) = 0;
        // }
        // make the user_sp aligned to 8B for k210 platform
        // user_sp -= user_sp % core::mem::size_of::<usize>();
        // initialize trap_cx
        trace!("kernel: exec .. initialize trap_cx");
        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            task.kstack.get_top(),
            trap_handler as usize,
        );
        trap_cx.x[10] = len;
        trap_cx.x[11] = user_sp;
        *task_inner.get_trap_cx() = trap_cx;
    }

    /// Only support processes with a single thread.
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        trace!("kernel: fork");
        let mut parent = self.inner_exclusive_access();
        assert_eq!(parent.thread_count(), 1);
        // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
        let memory_set = MemorySet::from_existed_user(&parent.memory_set);
        // alloc a pid
        let pid = pid_alloc();
        // copy fd table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
        // create child process pcb
        let child = Arc::new(Self {
            pid,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    deadlock_flag: false,
                    available: [0; 100],
                    available_sem: [0; 100],
                })
            },
        });
        // add child
        parent.children.push(Arc::clone(&child));
        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task(0)
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));
        // attach task to child process
        let mut child_inner = child.inner_exclusive_access();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);
        // modify kstack_top in trap_cx of this thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        trap_cx.kernel_sp = task.kstack.get_top();
        drop(task_inner);
        insert_into_pid2process(child.getpid(), Arc::clone(&child));
        // add this thread to scheduler
        add_task(task);
        child
    }
    /// get pid
    pub fn getpid(&self) -> usize {
        self.pid.0
    }
}
