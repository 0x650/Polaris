mod acpi;
mod apic;
pub mod asm;
pub mod crt;
mod e9;
pub mod entry;
mod gdt;
pub mod hpet;
mod idt;
pub mod intr;
mod mminit;
pub mod smp;
mod syscall_entry;

use crate::mm::var::VarProtectionFlags;
use bitflags::bitflags;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Context {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub isr: u64,
    pub error: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl Context {
    pub fn init(ip: usize, sp: usize, arg: usize, user: bool) -> Self {
        Self {
            rip: ip as u64,
            rsp: sp as u64,
            rdi: arg as u64,
            rflags: 0x202,
            cs: if user {
                gdt::SEL_USER_CODE
            } else {
                gdt::SEL_KERNEL_CODE
            } as u64,
            ss: if user {
                gdt::SEL_USER_DATA
            } else {
                gdt::SEL_KERNEL_DATA
            } as u64,
            ..Default::default()
        }
    }

    pub fn get_ip(&self) -> usize {
        self.rip as usize
    }

    pub fn set_ip(&mut self, ip: usize) {
        self.rip = ip as u64;
    }

    pub fn get_sp(&self) -> usize {
        self.rsp as usize
    }

    pub fn set_sp(&mut self, sp: usize) {
        self.rsp = sp as u64
    }

    pub fn get_ret(&self) -> usize {
        self.rax as usize
    }

    pub fn set_ret(&mut self, ret: usize) {
        self.rax = ret as u64
    }

    pub fn set_first_arg(&mut self, arg: usize) {
        self.rdi = arg as u64
    }

    pub fn get_first_arg(&self) -> usize {
        self.rdi as usize
    }

    pub fn get_syscall_nr(&self) -> usize {
        self.rax as usize
    }
}

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: usize = 12;
pub const STACK_SIZE: usize = PAGE_SIZE * 16;
pub const PFN_DATABASE: u64 = 0xFFFF_FA80_0000_0000;
pub const BIG_ALLOC_START: u64 = PFN_DATABASE + (1 << 40);
pub const STACK_ALLOCATIONS_START: u64 = PFN_DATABASE + (2 << 40);

bitflags! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct PteFlags: u64 {
        const PRESENT     = 1 << 0;
        const WRITABLE    = 1 << 1;
        const USER        = 1 << 2;
        const HUGE        = 1 << 7;
        const NO_EXECUTE  = 1 << 63;
    }
}

pub fn prot_to_pte_flags(flags: VarProtectionFlags) -> PteFlags {
    let mut pte_flags = PteFlags::USER;
    if flags.contains(VarProtectionFlags::READ) {
        pte_flags |= PteFlags::PRESENT;
    }
    if flags.contains(VarProtectionFlags::WRITE) {
        pte_flags |= PteFlags::WRITABLE;
    }
    if !flags.contains(VarProtectionFlags::EXECUTE) {
        pte_flags |= PteFlags::NO_EXECUTE;
    }
    pte_flags
}

pub fn request_yield() {
    apic::Lapic::send_ipi(apic::Lapic::get_id() as u32, 32);
}
