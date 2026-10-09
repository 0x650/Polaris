use super::SyscallFn;
use crate::arch::Context;
use crate::log;
use crate::mm::vmb;
use crate::sched;
use crate::sched::thread;
use crate::status_codes::PxStatus;

fn run<Args, F: SyscallFn<Args>>(f: F, ctx: &mut Context) {
    let status = f.call(ctx);
    ctx.set_ret(status as usize);
}

pub fn dispatch(context: &mut Context) {
    match context.get_syscall_nr() {
        super::SYSLOG => run(log::syscall_log, context),
        super::WAIT_FOR_SINGLE_OBJECT => {
            run(sched::dispatch::syscall_wait_on_single_object, context)
        }
        super::NEW_THREAD => run(thread::syscall_new_thread, context),
        super::TERMINATE_THREAD => run(thread::syscall_terminate_thread, context),
        super::NEW_VMB => run(vmb::syscall_new_vmb, context),
        super::MAP_VMB => run(vmb::syscall_map_vmb, context),
        n => {
            log!("Unknown syscall {n}");
            context.set_ret(PxStatus::InvalidArguments as usize);
        }
    }
}
