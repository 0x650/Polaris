use crate::arch::Context;
use crate::log;
use crate::status_codes::PxStatus;

pub fn dispatch(context: &mut Context) {
    let handler: fn(&mut Context) = match context.get_syscall_nr() {
        super::SYSLOG => log::syscall_log,

        n => {
            log!("Unknown syscall {n}");
            context.set_ret(PxStatus::InvalidArguments as usize);
            return;
        }
    };

    handler(context);
}
