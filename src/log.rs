use crate::{arch::Context, fbcon, locks::spinlock::SpinLock};
use core::fmt::{Display, Formatter, FormattingOptions};

pub struct DebugCon;

static LOG_LOCK: SpinLock<()> = SpinLock::new(());

pub fn log(msg: &dyn Display) {
    let _guard = LOG_LOCK.lock();
    unsafe {
        if let Some(fb) = &mut *&raw mut fbcon::FLANTERM_CTX {
            let mut fmt = Formatter::new(&mut ***fb, FormattingOptions::new());
            let _ = msg.fmt(&mut fmt);
        }
    }

    let mut con = DebugCon;
    let mut fmt = Formatter::new(&mut con, FormattingOptions::new());
    let _ = msg.fmt(&mut fmt);
}

#[macro_export]
macro_rules! log {
    ($($args: expr),+ $(,)?) => {
        $crate::log::log(&format_args!($($args),+))
    };
}

pub fn syscall_log(context: &mut Context) {
    let string = context.get_first_arg() as *const i8;

    if string.is_null() {
        return;
    }

    let c_string = unsafe { core::ffi::CStr::from_ptr(string) };
    log!("{}", c_string.to_str().unwrap());
}
