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
use core::fmt::Write;

#[repr(C)]
#[derive(Clone, Copy, Default)]
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

impl core::fmt::Debug for Context {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_char('\r')?;
        f.write_char('\n')?;
        f.write_fmt(format_args!(
            "RAX: {:016x} RBX: {:016x} RCX: {:016x} RDX: {:016x}\r\n",
            self.rax, self.rbx, self.rcx, self.rdx
        ))?;
        f.write_fmt(format_args!(
            "RBP: {:016x} RDI: {:016x} RSI: {:016x} R8 : {:016x}\r\n",
            self.rbp, self.rdi, self.rsi, self.r8
        ))?;
        f.write_fmt(format_args!(
            "R9 : {:016x} R10: {:016x} R11: {:016x} R12: {:016x}\r\n",
            self.r9, self.r10, self.r11, self.r12,
        ))?;
        f.write_fmt(format_args!(
            "R13: {:016x} R14: {:016x} R15: {:016x} RFL: {:016x}\r\n",
            self.r13, self.r14, self.r15, self.rflags
        ))?;
        f.write_fmt(format_args!(
            "RSP: {:016x} RIP: {:016x} CS : {:016x} SS : {:016x}",
            self.rsp, self.rip, self.cs, self.ss
        ))?;
        Ok(())
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
