use crate::arch::Context;
use crate::log;

pub fn dispatch(context: &mut Context) {
    let handler: fn(&mut Context) = match context.get_syscall_nr() {
        super::SYSLOG => log::syscall_log,

        n => {
            log!("Unknown syscall {n}");
            context.set_ret(67);
            return;
        }
    };

    handler(context);
}
