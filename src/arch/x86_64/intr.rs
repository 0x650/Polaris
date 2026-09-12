use super::Context;
use super::asm;
use super::smp;
use crate::mm::fault;
use crate::sched::sched;
use core::arch::{asm, global_asm};
use seq_macro;

pub unsafe fn enable_interrupts() {
    unsafe {
        asm::sti();
    }
}

pub unsafe fn disable_interrupts() {
    unsafe {
        asm::cli();
    }
}

pub unsafe fn get_interrupt_state() -> bool {
    unsafe {
        return asm::interrupt_state();
    }
}

pub unsafe fn toggle_interrupts(state: bool) -> bool {
    unsafe {
        return asm::toggle_interrupts(state);
    }
}

use core::arch::naked_asm;

seq_macro::seq!(N in 0..256 {
    #[unsafe(naked)]
    pub unsafe extern "C" fn interrupt_stub~N() {
        naked_asm!(
            ".if ({i} == 8 || ({i} >= 10 && {i} <= 14) || {i} == 17 || {i} == 21 || {i} == 29 || {i} == 30)",
            ".else",
            "push 0",
            ".endif",
            "push {i}",
            "jmp {internal}",
            i = const N,
            internal = sym interrupt_stub_internal,
        );
    }
});

#[unsafe(naked)]
unsafe extern "C" fn interrupt_stub_internal() {
    naked_asm!(
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rbp",
        "push rdi",
        "push rsi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "cld",
        "xor rbp, rbp",
        "mov rdi, rsp",
        "call {handler}",
        "jmp {ret}",
        handler = sym idt_handler,
        ret = sym interrupt_return,
    );
}

#[unsafe(naked)]
pub unsafe extern "C" fn interrupt_return() {
    naked_asm!(
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rsi",
        "pop rdi",
        "pop rbp",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",
        "add rsp, 0x10",
        "iretq",
    );
}

#[unsafe(naked)]
pub unsafe extern "C" fn load_context(context: &Context) {
    naked_asm!(
        "mov rsp, rdi",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rsi",
        "pop rdi",
        "pop rbp",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",
        "add rsp, 0x10",
        "iretq",
    );
}

use super::apic;

pub unsafe extern "C" fn idt_handler(context: *mut Context) {
    let context = unsafe { &mut *context };
    let isr = context.isr as u8;

    let prcb = smp::get_current_processor();
    let mut prcb = unsafe { &mut *prcb };

    match isr {
        0x00..0x1F => {
            if isr == 0x0E {
                let mut cr2: usize;
                unsafe { asm!("mov {cr2}, cr2", cr2 = out(reg) cr2) };
                fault::handle_fault(cr2 as u64);
            } else {
                panic!(
                    "Unhandled exception {}: {:#x?}\r\n{:?}\r\n",
                    isr, context.error, context
                );
            }
        }
        0x20 => {
            let next = sched::schedule(*context);
            apic::Lapic::eoi();
            match next {
                Some(next_thrd) => unsafe {
                    prcb.set_kernel_stack(&next_thrd.kernel_stack);
                    next_thrd.mother_proc.address_space.lock().set();
                    load_context(&next_thrd.context());
                },
                None => unsafe { load_context(context) },
            }
        }
        _ => {
            todo!()
        }
    }
}
