#![no_std]
#![no_main]
#![feature(formatting_options)]

use core::any::Any;
use core::panic::PanicInfo;
use core::sync::atomic::Ordering;
extern crate alloc;
extern crate core;
mod arch;
mod fbcon;
mod ipc;
mod locks;
mod log;
mod mm;
mod object;
mod sched;
mod status_codes;
mod syscall;

use arch::entry::MODULES_REQUEST;

use elf::ElfBytes;
use elf::abi::{ET_EXEC, PF_R, PF_W, PF_X, PT_LOAD};
use elf::endian::AnyEndian;

use mm::var::Var;
use mm::var::VarProtectionFlags;
use mm::vmb::Vmb;

use alloc::sync::Arc;

use crate::arch::PAGE_SIZE;
use crate::mm::virt::KERNEL_ADDRESS_SPACE;
use crate::mm::vmb::VmbBacking;

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    arch::entry::arch_entry();
}

extern "C" fn init_thread(_: usize) -> ! {
    log!("Hello from kernel init thread!\r\n");

    let user_elf = MODULES_REQUEST
        .response()
        .expect("Expected module response??")
        .modules()
        .iter()
        .find(|m| m.path().contains("user.elf"))
        .expect("Failed to find user.elf module");

    log!("Found user.elf at {}\r\n", user_elf.path());

    let elf_data = user_elf.data();
    let user_elf = ElfBytes::<AnyEndian>::minimal_parse(user_elf.data())
        .expect("Failed to parse user.elf binary??");

    if user_elf.ehdr.e_type != ET_EXEC {
        panic!("user.elf is not an ELF executable??");
    }

    let segments = user_elf.segments().expect("Failed to parse ELF segments??");
    let mut entry_point: usize = 0;

    let user_proc = sched::process::Process::new();

    const STACK_TOP: u64 = 0x0000_7000_0000_0000;
    const STACK_BASE: u64 = STACK_TOP - PAGE_SIZE as u64;

    {
        let mut address_space = user_proc.address_space.lock();
        address_space.setup_for_user();

        let stack_vmb = Arc::new(Vmb::new(PAGE_SIZE, VmbBacking::Anon));
        address_space
            .insert_var_range(
                STACK_BASE,
                PAGE_SIZE,
                VarProtectionFlags::WRITE | VarProtectionFlags::READ,
                stack_vmb,
            )
            .expect("Failed to map stack??");

        unsafe {
            address_space.set();
        }

        for phdr in segments {
            if phdr.p_type != PT_LOAD {
                continue;
            }

            let vaddr = phdr.p_vaddr as usize;
            let memsz = phdr.p_memsz as usize;
            let filesz = phdr.p_filesz as usize;
            let offset = phdr.p_offset as usize;

            let seg_vmb = Arc::new(Vmb::new_allocated_anon(memsz));
            address_space
                .map_var_range(vaddr as u64, memsz, VarProtectionFlags::all(), seg_vmb)
                .expect("Failed to map segment??");

            unsafe {
                let dest = vaddr as *mut u8;
                if filesz > 0 {
                    let src = &elf_data[offset..offset + filesz];
                    core::ptr::copy_nonoverlapping(src.as_ptr(), dest, filesz);
                }

                if memsz > filesz {
                    let zero_bytes = memsz - filesz;
                    core::ptr::write_bytes(dest.add(filesz), 0, zero_bytes);
                }
            }
        }

        unsafe {
            KERNEL_ADDRESS_SPACE.get().unwrap().lock().set();
        }
    }

    sched::sched::enqueue_thread(
        sched::thread::Thread::new(
            user_elf.ehdr.e_entry as usize,
            STACK_TOP as usize,
            0,
            user_proc.clone(),
        )
        .expect("Failed to create user thread!"),
    );

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
