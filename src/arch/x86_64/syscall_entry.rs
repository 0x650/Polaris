use super::smp::Prcb;
use crate::arch::Context;
use crate::mm::fault;
use crate::sched::sched;
use core::arch::naked_asm;
use core::arch::{asm, global_asm};
use core::mem::offset_of;

#[unsafe(naked)]
pub unsafe extern "C" fn amd64_syscall_stub() {
    naked_asm!(
        "swapgs",
        "mov gs:{user_stack}, rsp",
        "mov rsp, gs:{kernel_stack}",
        "cld",
        "push 0x1B",
        "push gs:{user_stack}",
        "push r11",
        "push 0x23",
        "push rcx",
        "push 0x00",
        "push 0x00",
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
        "xor rbp, rbp",
        "mov rdi, rsp",
        "sti",
        "call {syscall_handler}",
        "cli",
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
        "pop rcx",
        "add rsp, 8",
        "pop r11",
        "pop rsp",
        "swapgs",
        "sysretq",

        syscall_handler = sym syscall_handler,
        user_stack = const offset_of!(Prcb, user_stack),
        kernel_stack = const offset_of!(Prcb, kernel_stack),
    );
}

extern "C" fn syscall_handler(context: *mut Context) {
    unsafe {
        let context = context.as_mut().unwrap();
        crate::syscall::dispatch::dispatch(context);
    }
}
