use super::var;
use super::virt;
use crate::arch;
use crate::arch::PAGE_SIZE;
use crate::log;
use crate::mm::phys::*;
use crate::mm::vmb;
use crate::sched::process;
use crate::sched::thread;

fn align_down(x: u64, a: u64) -> u64 {
    x & !(a - 1)
}

pub fn handle_fault(faulting_address: u64) {
    let Some(running_thread) = arch::get_running_thread() else {
        panic!("Page fault at {:p}", faulting_address as *const ());
    };

    let addr_space = running_thread.mother_proc.address_space.clone();
    let mut addr_space = addr_space.lock();

    let Some((base_address, protections, vmb)) = addr_space
        .get_var_range(faulting_address)
        .map(|var| (var.base_address, var.protections, var.vmb.clone()))
    else {
        panic!("Page fault at {:p}", faulting_address as *const ());
    };

    addr_space.map(
        align_down(faulting_address, PAGE_SIZE as u64),
        vmb.get_page((faulting_address - base_address) as usize),
        arch::prot_to_pte_flags(protections),
    );
}
