use super::var::{self, Var, VarProtectionFlags};
use super::vmb::{self, Vmb, VmbBacking};
use crate::arch::{self, *};
use crate::locks::spinlock::SpinLock;
use crate::status_codes::{PxResult, PxStatus};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::AtomicU64;
use spin::Once;

pub static KERNEL_ADDRESS_SPACE: Once<Arc<SpinLock<AddressSpace>>> = Once::new();
pub static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

fn align_up(x: u64, a: u64) -> u64 {
    (x + a - 1) & !(a - 1)
}

fn align_down(x: u64, a: u64) -> u64 {
    x & !(a - 1)
}

pub(crate) struct PageTable {
    pub(crate) directory: u64,
}

pub struct AddressSpace {
    pub(crate) page_table: PageTable,
    vars: BTreeMap<u64, Var>,
}

impl AddressSpace {
    pub fn new() -> Self {
        Self {
            page_table: PageTable::new(),
            vars: BTreeMap::new(),
        }
    }

    pub fn map(&mut self, virt: u64, phys: u64, flags: PteFlags) {
        self.page_table.map(virt, phys, flags);
    }

    pub fn unmap(&mut self, virt: u64) -> Option<u64> {
        self.page_table.unmap(virt)
    }

    pub unsafe fn set(&self) {
        self.page_table.set();
    }

    pub fn map_var_range(
        &mut self,
        addr: u64,
        length: usize,
        protections: VarProtectionFlags,
        vmb: Arc<Vmb>,
    ) -> PxResult<()> {
        let status = self.insert_var_range(addr, length, protections, vmb.clone());
        if status.is_err() {
            return status;
        }

        let mut i = 0;
        while i < length {
            let page = match vmb.get_page(i) {
                Ok(p) => p,
                Err(status) => return Err(status),
            };

            self.map(
                i as u64 + align_down(addr, PAGE_SIZE as u64),
                page,
                arch::prot_to_pte_flags(protections),
            );

            i += PAGE_SIZE;
        }

        Ok(())
    }

    pub fn insert_var_range(
        &mut self,
        addr: u64,
        length: usize,
        protections: VarProtectionFlags,
        vmb: Arc<Vmb>,
    ) -> PxResult<()> {
        let base = align_down(addr, PAGE_SIZE as u64);
        let len = align_up(length as u64, PAGE_SIZE as u64) as usize;
        let end = base + len as u64;

        if let Some((&prev_base, prev_var)) = self.vars.range(..base).next_back() {
            if prev_base + prev_var.length as u64 > base {
                return Err(PxStatus::InvalidRange);
            }
        }

        if let Some((&next_base, _)) = self.vars.range(base..).next() {
            if next_base < end {
                return Err(PxStatus::AlreadyExists);
            }
        }

        self.vars
            .insert(base, Var::new(base, len, protections, vmb));

        Ok(())
    }

    pub fn get_var_range(&mut self, addr: u64) -> Option<&mut Var> {
        self.vars
            .range_mut(..=addr)
            .next_back()
            .filter(|(base, var)| addr < *base + var.length as u64)
            .map(|(_, var)| var)
    }

    pub fn remove_var_range(&mut self, addr: u64, length: usize) -> PxResult<()> {
        let addr = align_down(addr, PAGE_SIZE as u64);
        let length = align_up(length as u64, PAGE_SIZE as u64) as usize;
        let end = addr + length as u64;

        let Some((var_base, var_len, protections, vmb)) = self
            .get_var_range(addr)
            .map(|v| (v.base_address, v.length, v.protections, v.vmb.clone()))
        else {
            return Err(PxStatus::NotFound);
        };
        let var_end = var_base + var_len as u64;

        if end > var_end {
            return Err(PxStatus::InvalidRange);
        }

        self.vars.remove(&var_base);

        if var_base < addr {
            self.vars.insert(
                var_base,
                Var::new(
                    var_base,
                    (addr - var_base) as usize,
                    protections,
                    vmb.clone(),
                ),
            );
        }

        if end < var_end {
            self.vars.insert(
                end,
                Var::new(end, (var_end - end) as usize, protections, vmb.clone()),
            );
        }

        Ok(())
    }

    pub fn remap_var_range(
        &mut self,
        addr: u64,
        length: usize,
        protections: VarProtectionFlags,
    ) -> PxResult<()> {
        let addr = align_down(addr, PAGE_SIZE as u64);
        let length = align_up(length as u64, PAGE_SIZE as u64) as usize;
        let end = addr + length as u64;

        let Some((var_base, var_len, old_protections, vmb)) = self
            .get_var_range(addr)
            .map(|v| (v.base_address, v.length, v.protections, v.vmb.clone()))
        else {
            return Err(PxStatus::NotFound);
        };
        let var_end = var_base + var_len as u64;

        if end > var_end {
            return Err(PxStatus::InvalidRange);
        }

        self.vars.remove(&var_base);

        if var_base < addr {
            self.vars.insert(
                var_base,
                Var::new(
                    var_base,
                    (addr - var_base) as usize,
                    old_protections,
                    vmb.clone(),
                ),
            );
        }
        self.vars
            .insert(addr, Var::new(addr, length, protections, vmb.clone()));
        if end < var_end {
            self.vars.insert(
                end,
                Var::new(end, (var_end - end) as usize, old_protections, vmb.clone()),
            );
        }

        Ok(())
    }
}
