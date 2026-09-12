use super::apic;
use super::gdt;
use super::idt;
use super::syscall_entry;
use crate::arch::intr;
use crate::arch::x86_64::asm;
use crate::arch::x86_64::asm::interrupt_state;
use crate::arch::x86_64::gdt::CpuGdt;
use crate::locks::spinlock::SpinLock;
use crate::log;
use crate::mm::stack::*;
use crate::mm::virt::*;
use crate::sched::process;
use crate::sched::sched::RunQueue;
use crate::sched::thread::Thread;
use crate::sched::thread::idle_thread;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use core::array;
use core::mem::offset_of;
use core::sync::atomic::{AtomicUsize, Ordering};
use limine::mp::*;

const KERNEL_STACKS_COUNT: usize = 4;

pub struct Prcb {
    me: *mut Prcb,
    pub user_stack: u64,
    pub kernel_stack: u64,
    pub cpu_id: u32,
    gdt: gdt::CpuGdt,
    kernel_stacks: [KernelStack; KERNEL_STACKS_COUNT],
    pub ticks_in_10ms: u32,
    pub run_queue: SpinLock<RunQueue>,
    pub running_thread: Option<Arc<Thread>>,
}

unsafe impl Sync for Prcb {}

impl Prcb {
    pub fn set_kernel_stack(&mut self, kernel_stack: &KernelStack) {
        self.gdt.tss.rsp[0] = kernel_stack.top() as u64;
        self.kernel_stack = kernel_stack.top() as u64;
    }
}

pub static PROCESSOR_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static PROCESSORS: SpinLock<Vec<Option<&'static Prcb>>> = SpinLock::new(Vec::new());

pub fn get_current_processor() -> *mut Prcb {
    if unsafe { interrupt_state() } {
        panic!("Calling get_current_processor with interrupts enabled is a bug!\r\n");
    }

    let processor: *mut Prcb;
    unsafe {
        asm!(
            "mov {processor}, gs:[{me}]",
            processor = out(reg) processor,
            me = const offset_of!(Prcb, me),
            options(nostack, preserves_flags),
        );
    }
    processor
}

pub fn get_running_thread() -> Option<&'static Thread> {
    let previous = unsafe { intr::toggle_interrupts(false) };
    let prcb = unsafe { &mut *get_current_processor() };
    let running_thread = prcb.running_thread.as_deref();
    unsafe { intr::toggle_interrupts(previous) };
    running_thread
}

fn register_processor(prcb: &'static Prcb) {
    let mut processors = PROCESSORS.lock();
    processors[prcb.cpu_id as usize] = Some(prcb);
    PROCESSOR_COUNT.fetch_add(1, Ordering::SeqCst);
}

unsafe extern "C" fn processor_startup(mp_info: &MpInfo) -> ! {
    unsafe {
        intr::disable_interrupts();
    }

    unsafe {
        KERNEL_ADDRESS_SPACE.get().unwrap().lock().set();
    }

    let prcb = mp_info.extra_argument() as *mut Prcb;
    let mut prcb = unsafe { &mut *prcb };

    prcb.gdt.load();
    idt::load();

    unsafe {
        asm::wrmsr(0xC0000100, 0);
        asm::wrmsr(0xC0000101, mp_info.extra_argument());
        asm::wrmsr(0xC0000102, mp_info.extra_argument());

        let msr = asm::rdmsr(0xC0000080);
        asm::wrmsr(0xC0000080, msr | (1 << 0) as u64);
        asm::wrmsr(
            0xC0000081,
            (0x20 - 16) << 48 | (gdt::SEL_KERNEL_CODE as u64) << 32,
        );
        asm::wrmsr(
            0xC0000082,
            syscall_entry::amd64_syscall_stub as *const () as u64,
        );
        asm::wrmsr(0xC0000084, 0);
    }

    apic::Lapic::init(prcb.cpu_id as u8);

    log!("Hello I am processor {}\r\n", unsafe {
        (*get_current_processor()).cpu_id
    });

    register_processor(prcb);

    unsafe {
        intr::enable_interrupts();
    }
    loop {}
}

fn processor_setup_bsp(prcb: &'static mut Prcb) {
    unsafe {
        intr::disable_interrupts();
    }

    prcb.gdt.load();
    idt::load();

    unsafe {
        asm::wrmsr(0xC0000100, 0);
        asm::wrmsr(0xC0000101, (prcb as *mut Prcb) as u64);
        asm::wrmsr(0xC0000102, (prcb as *mut Prcb) as u64);

        let msr = asm::rdmsr(0xC0000080);
        asm::wrmsr(0xC0000080, msr | (1 << 0) as u64);
        asm::wrmsr(
            0xC0000081,
            (0x20 - 16) << 48 | (gdt::SEL_KERNEL_CODE as u64) << 32,
        );
        asm::wrmsr(
            0xC0000082,
            syscall_entry::amd64_syscall_stub as *const () as u64,
        );
        asm::wrmsr(0xC0000084, 0);
    }

    apic::Lapic::init(prcb.cpu_id as u8);

    log!("Hello I am the boot processor {}\r\n", unsafe {
        (*get_current_processor()).cpu_id
    });

    register_processor(prcb);
}

pub(in crate::arch::x86_64) fn init(mp_request: &MpRespData) {
    let processor_count = mp_request.cpus().len();

    let mut v = Vec::with_capacity(processor_count);
    v.resize_with(processor_count, || None);
    *PROCESSORS.lock() = v;

    for cpu in mp_request.cpus() {
        let mut kernel_stacks: [KernelStack; KERNEL_STACKS_COUNT] =
            array::from_fn(|_| KernelStack::new().expect("Failed to allocate kernel stack?"));

        let mut prcb = Box::new(Prcb {
            me: core::ptr::null_mut(),
            user_stack: 0,
            kernel_stack: 0,
            cpu_id: cpu.processor_id,
            gdt: CpuGdt::new(),
            kernel_stacks,
            ticks_in_10ms: 0,
            run_queue: SpinLock::new(RunQueue::new()),
            running_thread: None,
        });

        let prcb_ref: &'static mut Prcb = Box::leak(prcb);
        prcb_ref.me = prcb_ref as *mut Prcb;

        prcb_ref.gdt.tss.rsp[0] = prcb_ref.kernel_stacks[0].top() as u64;
        prcb_ref.kernel_stack = prcb_ref.kernel_stacks[0].top() as u64;

        prcb_ref.gdt.tss.ist[0] = prcb_ref.kernel_stacks[1].top() as u64;
        prcb_ref.gdt.tss.ist[1] = prcb_ref.kernel_stacks[2].top() as u64;
        prcb_ref.gdt.tss.ist[2] = prcb_ref.kernel_stacks[3].top() as u64;

        let idle_thread = Thread::new_kernel(idle_thread, 0, process::kernel_process().clone())
            .expect("Failed to create idle thread?");
        idle_thread.niceness.store(19, Ordering::Release);
        prcb_ref.run_queue.lock().enqueue(idle_thread);

        if cpu.lapic_id == mp_request.bsp_lapic_id {
            processor_setup_bsp(prcb_ref);
        } else {
            cpu.bootstrap(processor_startup, prcb_ref.me as u64);
        }
    }

    while PROCESSOR_COUNT.load(Ordering::Relaxed) != mp_request.cpus().len() {
        core::hint::spin_loop();
    }

    log!(
        "The system has {} processor(s) installed\r\n",
        PROCESSOR_COUNT.load(Ordering::Relaxed)
    );
}
