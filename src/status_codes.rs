use core::result::Result;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PxStatus {
    Success = 0,
    Unsuccessful = -1,
    InvalidRange = -2,
    FailedToAllocate = -3,
    NotFound = -4,
    AlreadyExists = -5,
    InvalidArguments = -6,
    NotSupported = -7,
    TimedOut = -8,
}

pub type PxResult<T> = Result<T, PxStatus>;
