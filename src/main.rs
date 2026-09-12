#![no_std]
#![no_main]
#![feature(formatting_options)]

use core::panic::PanicInfo;
use core::sync::atomic::Ordering;
extern crate alloc;
extern crate core;
mod arch;
mod fbcon;
mod locks;
mod mm;
mod object;
mod sched;
mod syscall;

use crate::{
    arch::{PAGE_SIZE, PteFlags},
    locks::mutex::Mutex,
    mm::{
        phys::PageUsage,
        var::{VarFlags, VarProtectionFlags},
    },
    object::handle::Handle,
    sched::{
        dispatch::{DispatcherObject, Event},
        thread::Thread,
    },
};
use spin::Once;

use alloc::sync::Arc;

#[macro_use]
mod log;

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    arch::entry::arch_entry();
}

extern "C" fn init_thread(_: usize) -> ! {
    log!("Hello from kernel init thread!\r\n");

    let user_proc = sched::process::Process::new();

    const STACK_TOP: u64 = 0x7F00_0000_0000_0000;
    const STACK_BASE: u64 = STACK_TOP - PAGE_SIZE as u64;
    const CODE_BASE: u64 = 0x0000_0000_4000_0000;
    {
        let address_space = user_proc.address_space.clone();
        let mut address_space = address_space.lock();
        address_space.setup_for_user();
        address_space.insert_var_range(
            STACK_BASE,
            PAGE_SIZE,
            VarProtectionFlags::WRITE | VarProtectionFlags::READ,
            VarFlags::ANON,
        );
        address_space.insert_var_range(
            CODE_BASE,
            PAGE_SIZE,
            VarProtectionFlags::READ | VarProtectionFlags::EXECUTE,
            VarFlags::ANON,
        );

        let p = mm::phys::PMM
            .lock()
            .as_mut()
            .unwrap()
            .alloc(PageUsage::Anon)
            .unwrap();
        address_space.map(CODE_BASE, p, PteFlags::USER | PteFlags::PRESENT);

        const SHELLCODE: [u8; 37] = [
            0x31, 0xc0, // xor rax, rax
            0x48, 0x8d, 0x3d, 0x04, 0x00, 0x00, 0x00, // lea rdi, [rip + 4]
            0x0f, 0x05, // syscall
            0xeb, 0xfe, // jmp $
            b'H', b'e', b'l', b'l', b'o', b' ', b'f', b'r', b'o', b'm', b' ', b'U', b's', b'e',
            b'r', b's', b'p', b'a', b'c', b'e', b'!', b'\r', b'\n', 0x00,
        ];

        unsafe {
            let dest = (p + mm::virt::HHDM_OFFSET.load(Ordering::Relaxed)) as *mut u8;
            core::ptr::copy_nonoverlapping(SHELLCODE.as_ptr(), dest, SHELLCODE.len());
        }
    }

    let user_thread =
        sched::thread::Thread::new(CODE_BASE as usize, STACK_TOP as usize, 0, user_proc.clone())
            .expect("Failed to create user thread!");
    sched::sched::enqueue_thread(user_thread.clone());

    loop {}
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    log!("*** PANIC!\r\n");
    if let Some(loc) = info.location() {
        log!("PANIC: {}:{}: ", loc.file(), loc.line());
    }
    log!("{}\r\n", info.message());
    loop {
        arch::asm::halt_forever();
    }
}
