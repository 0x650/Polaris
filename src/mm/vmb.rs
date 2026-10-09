use super::phys::{self, PMM, PageUsage};
use crate::arch::{PAGE_SHIFT, PAGE_SIZE, get_running_thread};
use crate::locks::spinlock::SpinLock;
use crate::mm::var::VarProtectionFlags;
use crate::object::KernelObject;
use crate::object::handle::{self, Handle};
use crate::sched::process::Process;
use crate::status_codes::{PxResult, PxStatus};
use crate::syscall::FromArg;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::result::Result;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VmbBacking {
    Anon,
    Pager,
}

impl FromArg for VmbBacking {
    fn from_arg(r: usize) -> PxResult<Self> {
        match r {
            0 => return Ok(VmbBacking::Anon),
            1 => return Ok(VmbBacking::Pager),
            _ => return Err(PxStatus::InvalidRange),
        }
    }
}

pub struct Vmb {
    pub size: usize,
    backing_type: VmbBacking,
    pages: SpinLock<BTreeMap<usize, u64>>,
}

fn align_up(x: usize, a: usize) -> usize {
    (x + a - 1) & !(a - 1)
}

impl Vmb {
    pub fn new(size: usize, backing_type: VmbBacking) -> Self {
        Self {
            size: align_up(size, PAGE_SIZE),
            backing_type,
            pages: SpinLock::new(BTreeMap::new()),
        }
    }

    pub fn new_allocated_anon(size: usize) -> Self {
        let v = Self {
            size: align_up(size, PAGE_SIZE),
            backing_type: VmbBacking::Anon,
            pages: SpinLock::new(BTreeMap::new()),
        };

        {
            let mut i = 0;
            let mut pmm = PMM.lock();
            let mut pages = v.pages.lock();
            while i < size {
                if let Some(page) = pmm.as_mut().unwrap().alloc(PageUsage::Anon) {
                    pages.insert(i >> PAGE_SHIFT, page);
                }
                i += PAGE_SIZE;
            }
        }

        v
    }

    pub fn get_page(&self, off: usize) -> PxResult<u64> {
        if off > self.size {
            return Err(PxStatus::InvalidRange);
        }

        let indx = off >> PAGE_SHIFT;

        if self.backing_type == VmbBacking::Pager {
            todo!();
        }

        let mut pages = self.pages.lock();
        if let Some(page) = pages.get(&indx) {
            return Ok(*page);
        }

        let mut pmm = PMM.lock();
        if let Some(page) = pmm.as_mut().unwrap().alloc(PageUsage::Anon) {
            pages.insert(indx, page);
            return Ok(page);
        }

        Err(PxStatus::FailedToAllocate)
    }
}

impl Drop for Vmb {
    fn drop(&mut self) {
        let mut pages = self.pages.lock();
        let mut pmm = PMM.lock();
        for (_, page) in pages.iter() {
            pmm.as_mut().unwrap().free(*page);
        }
        pages.clear();
    }
}

pub fn syscall_new_vmb(vmb_handle: *mut isize, length: usize, backing: VmbBacking) -> PxStatus {
    let running_proc = get_running_thread().unwrap().mother_proc.clone();

    if vmb_handle.is_null() {
        return PxStatus::BufferTooSmall;
    }

    let mut table = running_proc.handle_table.lock();
    let handle = table.insert(Handle::new(KernelObject::Vmb(Arc::new(Vmb::new(
        length, backing,
    )))));

    unsafe {
        *vmb_handle = handle;
    }

    PxStatus::Success
}

pub fn syscall_map_vmb(
    process_handle: isize,
    vmb_handle: isize,
    base: usize,
    length: usize,
    protections: VarProtectionFlags,
) -> PxStatus {
    let process = match handle::get_object_as::<Arc<Process>>(process_handle) {
        Ok(p) => p,
        Err(e) => {
            return e;
        }
    };

    let vmb = match handle::get_object_as::<Arc<Vmb>>(vmb_handle) {
        Ok(v) => v,
        Err(e) => {
            return e;
        }
    };

    let mut address_space = process.address_space.lock();
    match address_space.insert_var_range(base as u64, length, protections, vmb.clone()) {
        Ok(()) => {
            return PxStatus::Success;
        }
        Err(e) => {
            return e;
        }
    }
}
