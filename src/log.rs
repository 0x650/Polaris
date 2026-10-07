use crate::{arch::Context, fbcon, locks::spinlock::SpinLock, status_codes::PxStatus};
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

pub fn syscall_log(string: *const ()) -> PxStatus {
    if string.is_null() {
        return PxStatus::BufferTooSmall;
    }

    let c_string = unsafe { core::ffi::CStr::from_ptr(string as *const i8) };
    let str = match c_string.to_str() {
        Ok(s) => s,
        Err(e) => return PxStatus::Unsuccessful,
    };

    log!("{}", str);

    PxStatus::Success
}
