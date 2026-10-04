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
mod ipc;

use spin::Once;

use alloc::sync::Arc;

use mm::var::Var;
use mm::var::VarProtectionFlags;
use mm::vmb::Vmb;
use mm::vmb::VmbBacking;

use sched::thread::Thread;

use crate::ipc::channels::Channel;

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    arch::entry::arch_entry();
}

extern "C" fn yet_another_thread(raw_ptr: usize) -> ! {
    log!("Hello I am yet another thread going to listen on the channel\r\n");

    unsafe {
        let this: Arc<Channel> = Arc::from_raw(raw_ptr as *const Channel);

        let mut buffer: [u8; 1024] = [0; 1024];
        let res = this.receive(&mut buffer);
        let length = res.unwrap();

        let funny = str::from_utf8(&buffer[..length]).unwrap();

        log!("Received {}!\r\n", funny);
    }

    loop {}
}

extern "C" fn init_thread(_: usize) -> ! {
    log!("Hello from kernel init thread!\r\n");

    let mut channel = Channel::new_pair();

    let mut this = channel.0;

    sched::sched::enqueue_thread(
        Thread::new_kernel(
            yet_another_thread,
            Arc::into_raw(channel.1) as usize,
            arch::get_running_thread().unwrap().mother_proc.clone(),
        )
        .unwrap(),
    );

    sched::sched::yield_execution();

    this.send("Hello from init thread!".as_bytes());

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
