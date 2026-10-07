pub mod handle;

use crate::locks::mutex::KMutex;
use crate::mm::vmb::Vmb;
use crate::sched::dispatch::Event;
use crate::sched::process::Process;
use crate::sched::thread::Thread;
use alloc::sync::Arc;

#[derive(Clone)]
pub enum KernelObject {
    Event(Arc<Event>),
    Thread(Arc<Thread>),
    Process(Arc<Process>),
    Mutex(Arc<KMutex>),
    Vmb(Arc<Vmb>),
}

impl KernelObject {
    pub fn as_event(&self) -> Option<Arc<Event>> {
        match self {
            KernelObject::Event(ev) => Some(ev.clone()),
            _ => None,
        }
    }

    pub fn as_thread(&self) -> Option<Arc<Thread>> {
        match self {
            KernelObject::Thread(t) => Some(t.clone()),
            _ => None,
        }
    }

    pub fn as_process(&self) -> Option<Arc<Process>> {
        match self {
            KernelObject::Process(p) => Some(p.clone()),
            _ => None,
        }
    }

    pub fn as_mutex(&self) -> Option<Arc<KMutex>> {
        match self {
            KernelObject::Mutex(m) => Some(m.clone()),
            _ => None,
        }
    }
    pub fn as_vmb(&self) -> Option<Arc<Vmb>> {
        match self {
            KernelObject::Vmb(v) => Some(v.clone()),
            _ => None,
        }
    }
}
