use super::process::Process;
use super::sched;
use super::thread::{Thread, ThreadState};
use crate::arch;
use crate::locks::mutex::KMutex;
use crate::object::{KernelObject, handle};
use crate::status_codes::{PxResult, PxStatus};
use alloc::sync::Arc;
use core::result::Result;
use core::sync::atomic::{AtomicBool, Ordering};
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub trait Dispatcher {
    fn test(&self) -> bool;
}

impl<T: Dispatcher + ?Sized> Dispatcher for Arc<T> {
    fn test(&self) -> bool {
        (**self).test()
    }
}

pub struct Event {
    state: AtomicBool,
}

impl Event {
    pub fn new() -> Self {
        Self {
            state: AtomicBool::new(false),
        }
    }

    pub fn trigger(&self, state: bool) {
        self.state.store(state, Ordering::Release);
    }
}

impl Dispatcher for Event {
    fn test(&self) -> bool {
        self.state.load(Ordering::Acquire)
    }
}

#[enum_dispatch(Dispatcher)]
pub enum DispatcherObject {
    Event(Arc<Event>),
    Thread(Arc<Thread>),
    Process(Arc<Process>),
    Mutex(Arc<KMutex>),
}

impl From<DispatcherObject> for KernelObject {
    fn from(d: DispatcherObject) -> Self {
        match d {
            DispatcherObject::Event(e) => KernelObject::Event(e),
            DispatcherObject::Thread(t) => KernelObject::Thread(t),
            DispatcherObject::Process(p) => KernelObject::Process(p),
            DispatcherObject::Mutex(m) => KernelObject::Mutex(m),
        }
    }
}

pub fn wait_on_single_object(object: Arc<DispatcherObject>, timeout: usize) -> PxStatus {
    let running_thread =
        arch::get_running_thread().expect("wait_on_single_object called with no thread running??");

    if object.test() {
        return PxStatus::Success;
    }

    {
        let mut wait_state = running_thread.wait_state.lock();
        wait_state.waiting_objects.push(object.clone());
        wait_state.wait_time_out = timeout;
        wait_state.waiting_on_all = true;
        assert!(wait_state.waiting_objects.len() == 1);
    }

    *running_thread.state.lock() = ThreadState::Waiting;

    sched::yield_execution();

    running_thread.wait_state.lock().waiting_objects.clear();

    if object.test() {
        return PxStatus::Success;
    }

    PxStatus::TimedOut
}

pub fn wait_on_multiple_objects(
    objects: &[Arc<DispatcherObject>],
    wait_all: bool,
    timeout: usize,
) -> Result<Arc<DispatcherObject>, PxStatus> {
    let running_thread = arch::get_running_thread()
        .expect("wait_on_multiple_objects called with no thread running??");

    for object in objects {
        if object.test() && !wait_all {
            return Ok(object.clone());
        }
    }

    {
        let mut wait_state = running_thread.wait_state.lock();
        for object in objects {
            wait_state.waiting_objects.push(object.clone());
        }
        wait_state.wait_time_out = timeout;
        wait_state.waiting_on_all = wait_all;
        assert!(wait_state.waiting_objects.len() == objects.len());
    }

    *running_thread.state.lock() = ThreadState::Waiting;

    sched::yield_execution();

    let object = running_thread.test_objects(wait_all);

    running_thread.wait_state.lock().waiting_objects.clear();

    let obj = match object {
        Some(o) => o,
        None => return Err(PxStatus::TimedOut),
    };

    Ok(obj)
}

pub fn syscall_wait_on_single_object(handle: isize, timeout: usize) -> PxStatus {
    let running_proc = arch::get_running_thread().unwrap().mother_proc.clone();

    let object = match handle::get_object_as::<Arc<DispatcherObject>>(handle) {
        Ok(o) => o,
        Err(e) => {
            return e;
        }
    };

    wait_on_single_object(object, timeout)
}

pub fn syscall_wait_on_multiple_objects(
    handles: *const isize,
    number_of_handles: usize,
    wait_all: bool,
    timeout: usize,
) -> PxStatus {
    PxStatus::Unsuccessful
}
