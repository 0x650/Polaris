use super::dispatch::{Dispatcher, DispatcherObject};
use super::process::Process;
use super::sched;
use crate::mm::stack::*;
use crate::{arch::Context, locks::spinlock::SpinLock};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicI8, AtomicUsize, Ordering};
use intrusive_collections::{KeyAdapter, RBTreeLink, UnsafeRef, intrusive_adapter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Waiting,
    Terminating,
    Terminated,
}

static CURRENT_THREAD_ID: AtomicUsize = AtomicUsize::new(1);

pub struct WaitState {
    pub waiting_on_all: bool,
    pub waiting_objects: Vec<Arc<DispatcherObject>>,
    pub wait_time_out: usize,
}

pub struct Thread {
    pub id: usize,
    pub vruntime: AtomicUsize,
    pub niceness: AtomicI8,
    context: UnsafeCell<Context>,
    pub state: SpinLock<ThreadState>,
    pub last_scheduled_at: AtomicUsize,
    pub kernel_stack: KernelStack,
    pub wait_state: SpinLock<WaitState>,
    pub mother_proc: Arc<Process>,
    pub queued: AtomicBool,
    link: RBTreeLink,
}

unsafe impl Sync for Thread {}

intrusive_adapter!(pub ThreadAdapter = UnsafeRef<Thread>: Thread { link => RBTreeLink });

impl<'a> KeyAdapter<'a> for ThreadAdapter {
    type Key = (usize, usize);
    fn get_key(&self, thread: &'a Thread) -> (usize, usize) {
        (thread.vruntime.load(Ordering::Relaxed), thread.id)
    }
}

impl Thread {
    pub fn new_kernel(
        entry: extern "C" fn(usize) -> !,
        arg: usize,
        mother_proc: Arc<Process>,
    ) -> Option<Arc<Self>> {
        let kernel_stack = KernelStack::new()?;
        let sp = kernel_stack.top();

        let thread = Arc::new(Self {
            id: CURRENT_THREAD_ID.fetch_add(1, Ordering::SeqCst),
            vruntime: AtomicUsize::new(0),
            niceness: AtomicI8::new(0),
            state: SpinLock::new(ThreadState::Ready),
            context: UnsafeCell::new(Context::init(entry as usize, sp, arg, false)),
            last_scheduled_at: AtomicUsize::new(0),
            kernel_stack,
            wait_state: SpinLock::new(WaitState {
                waiting_on_all: false,
                waiting_objects: Vec::new(),
                wait_time_out: 0,
            }),
            mother_proc: mother_proc.clone(),
            queued: AtomicBool::new(false),
            link: RBTreeLink::new(),
        });

        mother_proc.add_thread(thread.clone());

        Some(thread.clone())
    }

    pub fn new(ip: usize, sp: usize, arg: usize, mother_proc: Arc<Process>) -> Option<Arc<Self>> {
        let kernel_stack = KernelStack::new()?;

        let thread = Arc::new(Self {
            id: CURRENT_THREAD_ID.fetch_add(1, Ordering::SeqCst),
            vruntime: AtomicUsize::new(0),
            niceness: AtomicI8::new(0),
            state: SpinLock::new(ThreadState::Ready),
            context: UnsafeCell::new(Context::init(ip, sp, arg, true)),
            last_scheduled_at: AtomicUsize::new(0),
            kernel_stack,
            wait_state: SpinLock::new(WaitState {
                waiting_on_all: false,
                waiting_objects: Vec::new(),
                wait_time_out: 0,
            }),
            mother_proc: mother_proc.clone(),
            queued: AtomicBool::new(false),
            link: RBTreeLink::new(),
        });

        mother_proc.add_thread(thread.clone());

        Some(thread.clone())
    }

    pub fn terminate(&self) {
        *self.state.lock() = ThreadState::Terminated;
        sched::yield_execution();
    }

    pub fn test_objects(&self, wait_all: bool) -> Option<Arc<DispatcherObject>> {
        let wait_state = self.wait_state.lock();
        if wait_all {
            for obj in &wait_state.waiting_objects {
                if !obj.test() {
                    return None;
                }
            }
            wait_state.waiting_objects.first().cloned()
        } else {
            wait_state
                .waiting_objects
                .iter()
                .find(|obj| obj.test())
                .cloned()
        }
    }

    pub unsafe fn context_mut(&self) -> &mut Context {
        unsafe { &mut *self.context.get() }
    }

    pub fn context(&self) -> &Context {
        unsafe { &*self.context.get() }
    }
}

impl Dispatcher for Thread {
    fn test(&self) -> bool {
        *self.state.lock() == ThreadState::Terminated
    }
}

pub extern "C" fn idle_thread(_: usize) -> ! {
    loop {}
}
