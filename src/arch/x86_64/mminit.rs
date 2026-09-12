use crate::arch::*;
use crate::locks::spinlock::SpinLock;
use crate::log;
use crate::mm::phys::*;
use crate::mm::virt::*;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use limine::memmap;

const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

#[repr(transparent)]
#[derive(Clone, Copy)]
struct Entry(u64);

impl Entry {
    fn is_present(&self) -> bool {
        self.0 & PteFlags::PRESENT.bits() != 0
    }
    fn addr(&self) -> u64 {
        self.0 & ADDR_MASK
    }
    fn set(&mut self, phys: u64, flags: PteFlags) {
        self.0 = (phys & ADDR_MASK) | flags.bits();
    }
}

#[repr(C, align(4096))]
struct Table {
    entries: [Entry; 512],
}

fn address_to_indices(virt: u64) -> (usize, usize, usize, usize) {
    (
        ((virt >> 39) & 0x1FF) as usize,
        ((virt >> 30) & 0x1FF) as usize,
        ((virt >> 21) & 0x1FF) as usize,
        ((virt >> 12) & 0x1FF) as usize,
    )
}

fn get_next_level(level: &mut Table, index: usize, alloc: bool) -> Option<*mut Table> {
    let entry = &mut level.entries[index];

    if entry.is_present() {
        return Some((entry.addr() + HHDM_OFFSET.load(Ordering::Relaxed)) as *mut Table);
    }

    if !alloc {
        return None;
    }

    let next_level_phys = PMM.lock().as_mut().unwrap().alloc(PageUsage::PageTable)?;

    unsafe {
        let next_level_ptr = (next_level_phys + HHDM_OFFSET.load(Ordering::Relaxed)) as *mut u8;
        core::ptr::write_bytes(next_level_ptr, 0, PAGE_SIZE);
    }

    entry.set(
        next_level_phys,
        PteFlags::PRESENT | PteFlags::WRITABLE | PteFlags::USER,
    );

    Some((next_level_phys + HHDM_OFFSET.load(Ordering::Relaxed)) as *mut Table)
}

impl PageTable {
    pub fn new() -> Self {
        let dir = PMM
            .lock()
            .as_mut()
            .unwrap()
            .alloc(PageUsage::PageTable)
            .expect("Failed to allocate page for page table?");
        unsafe {
            let dir_ptr = (dir + (HHDM_OFFSET.load(Ordering::Relaxed))) as *mut u8;
            core::ptr::write_bytes(dir_ptr, 0, PAGE_SIZE);
        }

        Self { directory: dir }
    }

    fn top_level(&self) -> &mut Table {
        unsafe { &mut *((self.directory + HHDM_OFFSET.load(Ordering::Relaxed)) as *mut Table) }
    }

    pub fn set(&self) {
        unsafe {
            core::arch::asm!("mov cr3, {}", in(reg) self.directory, options(nostack, preserves_flags));
        }
    }

    pub fn map(&mut self, virt: u64, phys: u64, flags: PteFlags) -> Option<()> {
        let (i4, i3, i2, i1) = address_to_indices(virt);

        let pdpt = unsafe { &mut *get_next_level(self.top_level(), i4, true)? };
        let pd = unsafe { &mut *get_next_level(pdpt, i3, true)? };
        let pt = unsafe { &mut *get_next_level(pd, i2, true)? };

        pt.entries[i1].set(phys, flags | PteFlags::PRESENT);

        Some(())
    }

    pub fn map_large(&mut self, virt: u64, phys: u64, flags: PteFlags) -> Option<()> {
        let (i4, i3, i2, _i1) = address_to_indices(virt);

        let pdpt = unsafe { &mut *get_next_level(self.top_level(), i4, true)? };
        let pd = unsafe { &mut *get_next_level(pdpt, i3, true)? };

        pd.entries[i2].set(phys, flags | PteFlags::PRESENT | PteFlags::HUGE);

        Some(())
    }

    pub fn unmap(&mut self, virt: u64) -> Option<u64> {
        let (i4, i3, i2, i1) = address_to_indices(virt);

        let pdpt = unsafe { &mut *get_next_level(self.top_level(), i4, false)? };
        let pd = unsafe { &mut *get_next_level(pdpt, i3, false)? };
        let pt = unsafe { &mut *get_next_level(pd, i2, false)? };

        let entry = &mut pt.entries[i1];
        if !entry.is_present() {
            return None;
        }

        let phys = entry.addr();
        *entry = Entry(0);

        Some(phys)
    }

    pub fn remap(&mut self, virt: u64, flags: PteFlags) -> Option<()> {
        let (i4, i3, i2, i1) = address_to_indices(virt);

        let pdpt = unsafe { &mut *get_next_level(self.top_level(), i4, false)? };
        let pd = unsafe { &mut *get_next_level(pdpt, i3, false)? };
        let pt = unsafe { &mut *get_next_level(pd, i2, false)? };

        let entry = &mut pt.entries[i1];
        if !entry.is_present() {
            return None;
        }

        let phys = entry.addr();
        entry.set(phys, flags | PteFlags::PRESENT);

        Some(())
    }
}

impl AddressSpace {
    pub fn setup_for_user(&mut self) {
        let kernel_space = KERNEL_ADDRESS_SPACE
            .get()
            .expect("Kernel address space is still not setup??");
        let kernel_table = kernel_space.lock();

        let from = kernel_table.page_table.top_level();
        let to = self.page_table.top_level();

        to.entries[256..512].copy_from_slice(&from.entries[256..512]);
    }
}

unsafe extern "C" {
    static __start_text: [u8; 0];
    static __stop_text: [u8; 0];
    static __start_rodata: [u8; 0];
    static __stop_rodata: [u8; 0];
    static __start_data: [u8; 0];
    static __stop_data: [u8; 0];
}

fn align_down(x: u64, align: u64) -> u64 {
    x & !(align - 1)
}
fn align_up(x: u64, align: u64) -> u64 {
    (x + align - 1) & !(align - 1)
}

pub(in crate::arch::x86_64) fn init(
    entries: &[&memmap::Entry],
    hhdm_offset: u64,
    phys_kernel_base: u64,
    virt_kernel_base: u64,
) {
    let highest_addr = entries
        .iter()
        .filter(|e| e.type_ == memmap::MEMMAP_USABLE)
        .map(|e| e.base + e.length)
        .max()
        .expect("No usable memory entries??");

    let highest_pfn = highest_addr.div_ceil(PAGE_SIZE as u64) as usize;
    let bitmap_size = highest_pfn.div_ceil(8) as usize;
    let pfndb_size = core::mem::size_of::<Page>() * highest_pfn;

    let region = entries
        .iter()
        .find(|e| e.type_ == memmap::MEMMAP_USABLE && e.length >= (bitmap_size + pfndb_size) as u64)
        .expect("No usable range to store pfndb + bitmap??");

    let pfndb_phys_base = region.base;
    let pfndb_virt_base = (pfndb_phys_base + hhdm_offset) as *mut u8;

    let pfndb = unsafe {
        let pfndb_ptr = pfndb_virt_base as *mut Page;
        for i in 0..highest_pfn {
            pfndb_ptr.add(i).write(Page {
                usage: PageUsage::Reserved,
                reference_count: AtomicUsize::new(0),
            });
        }
        core::slice::from_raw_parts_mut(pfndb_ptr, highest_pfn)
    };

    let bitmap = unsafe {
        let bitmap_ptr = pfndb_virt_base.add(pfndb_size) as *mut u8;
        core::ptr::write_bytes(bitmap_ptr, 0xFF, bitmap_size);
        core::slice::from_raw_parts_mut(bitmap_ptr, bitmap_size)
    };

    let mut total_page_count = 0;

    let mut this = PageAllocator::init(bitmap, pfndb, total_page_count, 0);

    for entry in entries.iter().filter(|e| e.type_ == memmap::MEMMAP_USABLE) {
        let start_pfn = entry.base >> PAGE_SHIFT;
        let end_pfn = (entry.base + entry.length) >> PAGE_SHIFT;

        for pfn in start_pfn..end_pfn {
            this.clear_bit(pfn as usize);
            this.pfndb[pfn as usize].usage = PageUsage::Free;
            total_page_count += 1;
        }
    }

    let pfndb_phys_start_pfn = pfndb_phys_base >> PAGE_SHIFT;
    let pfndb_phys_end_pfn =
        (pfndb_phys_base as usize + bitmap_size + pfndb_size).div_ceil(PAGE_SIZE) as u64;

    for pfn in pfndb_phys_start_pfn..pfndb_phys_end_pfn {
        this.set_bit(pfn as usize);
        this.pfndb[pfn as usize].usage = PageUsage::Reserved;
        total_page_count -= 1;
    }

    this.total_pages = total_page_count;

    HHDM_OFFSET.store(hhdm_offset, Ordering::Relaxed);

    *PMM.lock() = Some(this);

    let mut address_space = AddressSpace::new();

    for i in 256..512usize {
        get_next_level(address_space.page_table.top_level(), i, true).unwrap();
    }

    let hhdm = HHDM_OFFSET.load(Ordering::Relaxed);

    let mut p = 0u64;
    while p < 0x100000000 {
        address_space
            .page_table
            .map_large(p + hhdm, p, PteFlags::PRESENT | PteFlags::WRITABLE)
            .unwrap();
        p += 0x200000;
    }

    for entry in entries {
        let mut base = entry.base;
        let top = entry.base + entry.length;

        if base < 0x100000000 {
            base = 0x100000000;
        }
        if base >= top {
            continue;
        }

        let aligned_base = align_down(base, 0x200000 as u64);
        let aligned_top = align_up(top, 0x200000 as u64);

        let mut page = aligned_base;
        while page < aligned_top {
            address_space.page_table.map_large(
                page + hhdm,
                page,
                PteFlags::PRESENT | PteFlags::WRITABLE,
            );
            page += 0x200000;
        }
    }

    unsafe {
        let text_start = align_down(&__start_text as *const u8 as u64, PAGE_SIZE as u64);
        let text_end = align_up(&__stop_text as *const u8 as u64, PAGE_SIZE as u64);
        let rodata_start = align_down(&__start_rodata as *const u8 as u64, PAGE_SIZE as u64);
        let rodata_end = align_up(&__stop_rodata as *const u8 as u64, PAGE_SIZE as u64);
        let data_start = align_down(&__start_data as *const u8 as u64, PAGE_SIZE as u64);
        let data_end = align_up(&__stop_data as *const u8 as u64, PAGE_SIZE as u64);

        let mut addr = text_start;
        while addr < text_end {
            let phys = (addr - virt_kernel_base) + phys_kernel_base;
            address_space.map(addr, phys, PteFlags::PRESENT);
            addr += PAGE_SIZE as u64;
        }

        let mut addr = rodata_start;
        while addr < rodata_end {
            let phys = (addr - virt_kernel_base) + phys_kernel_base;
            address_space.map(addr, phys, PteFlags::PRESENT | PteFlags::NO_EXECUTE);
            addr += PAGE_SIZE as u64;
        }

        let mut addr = data_start;
        while addr < data_end {
            let phys = (addr - virt_kernel_base) + phys_kernel_base;
            address_space.map(
                addr,
                phys,
                PteFlags::PRESENT | PteFlags::WRITABLE | PteFlags::NO_EXECUTE,
            );
            addr += PAGE_SIZE as u64;
        }
    }

    p = PFN_DATABASE;
    let mut p_phys = pfndb_phys_base;
    while p < PFN_DATABASE + (pfndb_size + bitmap_size) as u64 {
        address_space.map(
            p,
            p_phys,
            PteFlags::PRESENT | PteFlags::WRITABLE | PteFlags::NO_EXECUTE,
        );
        p += PAGE_SIZE as u64;
        p_phys += PAGE_SIZE as u64;
    }

    {
        let mut pmm = PMM.lock();
        let pmm = pmm.as_mut().unwrap();

        pmm.pfndb =
            unsafe { core::slice::from_raw_parts_mut(PFN_DATABASE as *mut Page, highest_pfn) };
        pmm.bitmap = unsafe {
            core::slice::from_raw_parts_mut(
                (PFN_DATABASE + pfndb_size as u64) as *mut u8,
                bitmap_size,
            )
        };
    }

    unsafe {
        address_space.set();
    }

    KERNEL_ADDRESS_SPACE.call_once(|| Arc::new(SpinLock::new(address_space)));

    {
        let mut pmm = PMM.lock();
        let pmm = pmm.as_mut().unwrap();
        log!("The system has {} pages available\r\n", total_page_count);
        log!(
            "The kernel is loaded at {:p}\r\n",
            virt_kernel_base as *const ()
        );
        log!(
            "The HHDM offset is at {:p}\r\n",
            HHDM_OFFSET.load(Ordering::Relaxed) as *const ()
        );
        log!("The PFNDB is located at {:p}\r\n", pmm.pfndb.as_ptr());
    }
}
