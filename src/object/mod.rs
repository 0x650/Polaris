pub mod handle;

use crate::locks::mutex::KMutex;
use crate::mm::vmb::Vmb;
use crate::sched::dispatch::{DispatcherObject, Event};
use crate::sched::process::Process;
use crate::sched::thread::Thread;
use crate::status_codes::PxStatus;
use alloc::sync::Arc;

#[derive(Clone)]
pub enum KernelObject {
    Event(Arc<Event>),
    Thread(Arc<Thread>),
    Process(Arc<Process>),
    Mutex(Arc<KMutex>),
    Vmb(Arc<Vmb>),
}

macro_rules! kernel_object_conv {
    ($($variant:ident($ty:ty)),* $(,)?) => {$(
        impl From<Arc<$ty>> for KernelObject {
            fn from(v: Arc<$ty>) -> Self { KernelObject::$variant(v) }
        }
        impl TryFrom<KernelObject> for Arc<$ty> {
            type Error = PxStatus;
            fn try_from(o: KernelObject) -> Result<Self, PxStatus> {
                match o {
                    KernelObject::$variant(v) => Ok(v),
                    _ => Err(PxStatus::TypeMismatch),
                }
            }
        }
    )*};
}

kernel_object_conv!(
    Event(Event),
    Thread(Thread),
    Process(Process),
    Mutex(KMutex),
    Vmb(Vmb),
);

impl TryFrom<KernelObject> for Arc<DispatcherObject> {
    type Error = PxStatus;
    fn try_from(o: KernelObject) -> Result<Self, PxStatus> {
        match o {
            KernelObject::Event(e) => Ok(DispatcherObject::Event(e.clone()).into()),
            KernelObject::Thread(t) => Ok(DispatcherObject::Thread(t.clone()).into()),
            KernelObject::Process(p) => Ok(DispatcherObject::Process(p.clone()).into()),
            KernelObject::Mutex(m) => Ok(DispatcherObject::Mutex(m.clone()).into()),
            KernelObject::Vmb(_) => Err(PxStatus::TypeMismatch),
        }
    }
}
