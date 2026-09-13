use super::phys::{self, PMM, PageUsage};
use crate::arch::{PAGE_SHIFT, PAGE_SIZE};
use crate::locks::spinlock::SpinLock;
use crate::status_codes::PxStatus;
use alloc::collections::BTreeMap;
use core::result::Result;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VmbBacking {
    Anon,
    Pager,
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

    pub fn get_page(&self, off: usize) -> Result<u64, PxStatus> {
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
