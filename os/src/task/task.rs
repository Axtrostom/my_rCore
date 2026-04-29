//!Implementation of [`TaskControlBlock`]
use super::TaskContext;
use super::{KernelStack, PidHandle, pid_alloc};
use crate::config::TRAP_CONTEXT;
use crate::mm::{KERNEL_SPACE, MemorySet, PhysPageNum, VirtAddr};
use crate::sync::UPSafeCell;
use crate::trap::{TrapContext, trap_handler};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::cell::RefMut;

pub struct TaskControlBlock {
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
    //为什么这个要用 UPSafeCell 包一下？
    //
    //为什么不把这个 TaskControlBlock 的全部信息都包在 TaskControlBlockInner 里面？
    //因为别的信息 比如说 pid 和 kernel_stack 都是只读的

}

// 任务控制块的内部结构，包含了任务的状态、内存空间、父子关系等信息
pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum,//trap context 的物理页号
    #[allow(unused)]
    pub base_size: usize,
    pub task_cx: TaskContext,//保存了任务切换时的上下文信息
    pub task_status: TaskStatus,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<TaskControlBlock>>,//父进程的弱引用
    pub children: Vec<Arc<TaskControlBlock>>,//子进程的强引用
    pub exit_code: i32,
}

impl TaskControlBlockInner {
    /*
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }
    */
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    // 通过 elf 文件创建一个新的任务控制块
    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
    
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);//开辟空间并且装进去
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        //保存 trap context 的物理页号
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();//分配 pid
        let kernel_stack = KernelStack::new(&pid_handle);//通过 pid 分配内核栈
        let kernel_stack_top = kernel_stack.get_top();//内核栈栈顶地址
        // push a task context which goes to trap_return to the top of kernel stack
        let task_control_block = Self {//创建任务控制块
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                })
            },
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();//得到 trap context 的可变引用
        *trap_cx = TrapContext::app_init_context(//初始化 trap context
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }

    // 通过 elf 文件创建一个新的任务控制块，作为当前进程的子进程
    pub fn exec(&self, elf_data: &[u8]) {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);//通过 elf 创建内存空间并存入数据
        let trap_cx_ppn = memory_set//获取新建的 memoryset 中的 trap context 的物理页号
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // **** access inner exclusively
        let mut inner = self.inner_exclusive_access();//独占访问任务控制块的内部结构，下面是替换各个数据
        // substitute memory_set
        inner.memory_set = memory_set;//替换内存空寂
        // update trap_cx ppn
        inner.trap_cx_ppn = trap_cx_ppn;//替换上下文物理页号
        // initialize base_size
        inner.base_size = user_sp;//替换
        // initialize trap_cx
        let trap_cx = inner.get_trap_cx();//得到 trap context 的可变引用
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        // **** release inner automatically
    }

    pub fn fork(self: &Arc<Self>) -> Arc<Self> {//创建一个新的任务控制块，作为当前进程的子进程
        // ---- access parent PCB exclusively
        let mut parent_inner = self.inner_exclusive_access();//父进程的 PCB 独占访问
        // copy user space(include trap context)
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);//复制创建内存空间
        
        let trap_cx_ppn = memory_set//获取新建的 memoryset 中的 trap context 的物理页号
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();//分配 pid
        let kernel_stack = KernelStack::new(&pid_handle);//通过 pid 创建内核栈
        let kernel_stack_top = kernel_stack.get_top();//内核栈栈顶地址
        let task_control_block = Arc::new(TaskControlBlock {//创建新的任务控制块
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,//直接跟父进程一样的用户栈大小
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),//上下文信息
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),//建立对父进程的弱引用
                    children: Vec::new(),//子进程列表
                    exit_code: 0,//退出码
                })
            },
        });
        // add child
        parent_inner.children.push(task_control_block.clone());//把新建的任务控制块加入父进程的子进程列表
        // modify kernel_sp in trap_cx
        // **** access children PCB exclusively
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();//获得子进程上下文的可变引用
        trap_cx.kernel_sp = kernel_stack_top;//将子进程的内核栈指针切换到子进程的内核栈，防止父子进程切换时内核栈混乱
        // return
        task_control_block
        // ---- release parent PCB automatically
        // **** release children PCB automatically
    }
    pub fn getpid(&self) -> usize {
        self.pid.0
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}
