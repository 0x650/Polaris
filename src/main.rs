#![no_std]
#![no_main]
#![feature(formatting_options)]

use core::panic::PanicInfo;

extern crate alloc;
extern crate core;
mod arch;
mod fbcon;
mod locks;
mod mm;
mod object;
mod sched;

use crate::{
    arch::PAGE_SIZE,
    locks::mutex::Mutex,
    mm::var::{VarFlags, VarProtectionFlags},
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
    let running_thread = arch::get_running_thread().unwrap();

    const RANDOM_ADDRESS: u64 = 0xFFFFE00000000000;
    running_thread
        .mother_proc
        .address_space
        .lock()
        .insert_var_range(
            RANDOM_ADDRESS,
            PAGE_SIZE,
            VarProtectionFlags::READ | VarProtectionFlags::WRITE,
            VarFlags::ANON,
        );

    let funny_string = "Very funny";
    unsafe {
        let funny: &mut [u8] =
            core::slice::from_raw_parts_mut(RANDOM_ADDRESS as *mut u8, PAGE_SIZE);
        funny[..funny_string.len()].copy_from_slice(funny_string.as_bytes());
        log!(
            "Reading {:p}: {}\r\n",
            funny.as_ptr(),
            core::str::from_utf8(&funny[..funny_string.len()]).unwrap()
        );
    }

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
