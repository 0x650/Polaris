use crate::arch::PAGE_SHIFT;
use crate::locks::spinlock::SpinLock;
use core::sync::atomic::{AtomicUsize, Ordering};

pub enum PageUsage {
    Reserved,
    Free,
    PageTable,
    KernelHeap,
    KernelStack,
    Anon,
}

pub struct Page {
    pub(crate) usage: PageUsage,
    pub reference_count: AtomicUsize,
}

pub struct PageAllocator {
    pub(crate) bitmap: &'static mut [u8],
    pub(crate) pfndb: &'static mut [Page],
    pub total_pages: usize,
    pub(crate) last_freed_hint: usize,
}

pub static PMM: SpinLock<Option<PageAllocator>> = SpinLock::new(None);

impl PageAllocator {
    pub fn init(
        bitmap: &'static mut [u8],
        pfndb: &'static mut [Page],
        total_pages: usize,
        last_freed_hint: usize,
    ) -> Self {
        Self {
            bitmap,
            pfndb,
            total_pages,
            last_freed_hint,
        }
    }

    #[inline]
    pub(crate) fn test_bit(&self, pfn: usize) -> bool {
        (self.bitmap[pfn / 8] & (1 << (pfn % 8))) != 0
    }

    #[inline]
    pub(crate) fn set_bit(&mut self, pfn: usize) {
        self.bitmap[pfn / 8] |= 1 << (pfn % 8);
    }

    #[inline]
    pub(crate) fn clear_bit(&mut self, pfn: usize) {
        self.bitmap[pfn / 8] &= !(1 << (pfn % 8));
    }

    pub fn alloc(&mut self, usage: PageUsage) -> Option<u64> {
        for pfn in self.last_freed_hint..self.total_pages {
            if !self.test_bit(pfn) {
                self.set_bit(pfn);
                self.pfndb[pfn].usage = usage;
                self.pfndb[pfn].reference_count.store(1, Ordering::Relaxed);
                self.last_freed_hint = pfn + 1;
                return Some((pfn << PAGE_SHIFT) as u64);
            }
        }

        for pfn in 0..self.last_freed_hint {
            if !self.test_bit(pfn) {
                self.set_bit(pfn);
                self.pfndb[pfn].usage = usage;
                self.pfndb[pfn].reference_count.store(1, Ordering::Relaxed);
                self.last_freed_hint = pfn + 1;
                return Some((pfn << PAGE_SHIFT) as u64);
            }
        }
        None
    }

    pub fn free(&mut self, phys_addr: u64) {
        let pfn = (phys_addr as usize) >> PAGE_SHIFT;
        let prev = self.pfndb[pfn]
            .reference_count
            .fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            self.clear_bit(pfn);
            self.pfndb[pfn].usage = PageUsage::Free;
            if pfn < self.last_freed_hint {
                self.last_freed_hint = pfn;
            }
        }
    }

    pub fn ref_page(&self, phys_addr: u64) {
        let pfn = (phys_addr as usize) >> PAGE_SHIFT;
        self.pfndb[pfn]
            .reference_count
            .fetch_add(1, Ordering::AcqRel);
    }
}
