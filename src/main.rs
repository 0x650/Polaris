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
mod log;
mod mm;
mod object;
mod sched;
mod status_codes;
mod syscall;

use spin::Once;

use alloc::sync::Arc;

use mm::var::Var;
use mm::var::VarProtectionFlags;
use mm::vmb::Vmb;
use mm::vmb::VmbBacking;

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    arch::entry::arch_entry();
}

extern "C" fn init_thread(_: usize) -> ! {
    log!("Hello from kernel init thread!\r\n");

    let vmb = Arc::new(Vmb::new(arch::PAGE_SIZE, VmbBacking::Anon));

    let running_thread = arch::get_running_thread().unwrap();

    const RANDOM_ADDRESS: u64 = 0xFFFFE00000000000;
    running_thread
        .mother_proc
        .address_space
        .lock()
        .insert_var_range(
            RANDOM_ADDRESS,
            arch::PAGE_SIZE,
            VarProtectionFlags::READ | VarProtectionFlags::WRITE,
            vmb.clone(),
        );

    let wawa_string = "Wawawawa I love kiwawa wawawawa";
    unsafe {
        let wawa: &mut [u8] =
            core::slice::from_raw_parts_mut(RANDOM_ADDRESS as *mut u8, arch::PAGE_SIZE);
        wawa[..wawa_string.len()].copy_from_slice(wawa_string.as_bytes());
        log!(
            "Reading {:p}: {}\r\n",
            wawa.as_ptr(),
            core::str::from_utf8(&wawa[..wawa_string.len()]).unwrap()
        );
    }

    loop {}
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    arch::halt_other_processors();
    log!("*** PANIC!\r\n");
    if let Some(loc) = info.location() {
        log!("PANIC: {}:{}: ", loc.file(), loc.line());
    }
    log!("{}\r\n", info.message());
    loop {
        arch::asm::halt_forever();
    }
}
