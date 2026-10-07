pub mod dispatch;

use crate::arch::Context;
use crate::status_codes::{PxResult, PxStatus};

pub trait FromArg: Sized {
    fn from_arg(r: usize) -> PxResult<Self>;
}

impl FromArg for usize {
    fn from_arg(r: usize) -> PxResult<Self> {
        Ok(r)
    }
}

impl FromArg for isize {
    fn from_arg(r: usize) -> PxResult<Self> {
        Ok(r as isize)
    }
}

impl FromArg for u32 {
    fn from_arg(r: usize) -> PxResult<Self> {
        Ok(r as u32)
    }
}

impl FromArg for i32 {
    fn from_arg(r: usize) -> PxResult<Self> {
        Ok(r as i32)
    }
}

impl FromArg for u64 {
    fn from_arg(r: usize) -> PxResult<Self> {
        Ok(r as u64)
    }
}

impl FromArg for i64 {
    fn from_arg(r: usize) -> PxResult<Self> {
        Ok(r as i64)
    }
}

impl<T> FromArg for *const T {
    fn from_arg(r: usize) -> PxResult<Self> {
        Ok(r as *const T)
    }
}

impl<T> FromArg for *mut T {
    fn from_arg(r: usize) -> PxResult<Self> {
        Ok(r as *mut T)
    }
}

pub trait SyscallFn<Args> {
    fn call(&self, ctx: &Context) -> PxStatus;
}

macro_rules! impl_syscall_fn {
    ($($idx:tt $T:ident),*) => {
        #[allow(non_snake_case, unused_variables)]
        impl<F, $($T),*> SyscallFn<($($T,)*)> for F
        where
            F: Fn($($T),*) -> PxStatus,
            $($T: FromArg),*
        {
            fn call(&self, ctx: &Context) -> PxStatus {
                $(
                    let $T = match $T::from_arg(ctx.get_syscall_arg($idx)) {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                )*
                (self)($($T),*)
            }
        }
    };
}

impl_syscall_fn!();
impl_syscall_fn!(0 A);
impl_syscall_fn!(0 A, 1 B);
impl_syscall_fn!(0 A, 1 B, 2 C);
impl_syscall_fn!(0 A, 1 B, 2 C, 3 D);
impl_syscall_fn!(0 A, 1 B, 2 C, 3 D, 4 E);
impl_syscall_fn!(0 A, 1 B, 2 C, 3 D, 4 E, 5 G);

pub const SYSLOG: usize = 0;
pub const NEW_THREAD: usize = 1;
pub const NEW_VMB: usize = 2;
pub const MAP_VMB: usize = 3;
