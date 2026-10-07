use crate::object::KernelObject;
use crate::sched::dispatch::DispatcherObject;
use crate::status_codes::{PxResult, PxStatus};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

pub struct Handle {
    object: KernelObject,
}

pub struct HandleTable {
    handles: BTreeMap<isize, Handle>,
    next_id: isize,
}

impl HandleTable {
    pub fn new() -> Self {
        Self {
            handles: BTreeMap::new(),
            next_id: 4,
        }
    }

    pub fn insert(&mut self, handle: Handle) -> isize {
        let r = self.next_id;
        self.handles.insert(self.next_id, handle);
        self.next_id += 4;
        r
    }

    pub fn get(&mut self, handle_id: isize) -> Option<&Handle> {
        self.handles.get(&handle_id)
    }
}

impl Handle {
    pub fn new(object: KernelObject) -> Self {
        Self { object }
    }

    pub fn get(&self) -> KernelObject {
        self.object.clone()
    }
}

pub fn get_object(handle_id: isize) -> PxResult<KernelObject> {
    let running_thread =
        crate::arch::get_running_thread().expect("get_object called with no thread running??");

    if handle_id == -1 {
        let running_proc = running_thread.mother_proc.clone();
        return Ok(KernelObject::Process(running_proc));
    }

    if handle_id == -2 {
        return Ok(KernelObject::Thread(running_thread.clone()));
    }

    let object = {
        let mut table = running_thread.mother_proc.handle_table.lock();
        table.get(handle_id).ok_or(PxStatus::NotFound)?.get()
    };
    Ok(object)
}
