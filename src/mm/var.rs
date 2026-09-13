use super::vmb::Vmb;
use alloc::sync::Arc;
use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct VarProtectionFlags: i32 {
        const READ     = 1 << 0;
        const WRITE    = 1 << 1;
        const EXECUTE  = 1 << 2;
    }
}

pub struct Var {
    pub base_address: u64,
    pub length: usize,
    pub protections: VarProtectionFlags,
    pub vmb: Arc<Vmb>,
}

impl Var {
    pub fn new(base: u64, length: usize, protections: VarProtectionFlags, vmb: Arc<Vmb>) -> Self {
        Self {
            base_address: base,
            length,
            protections,
            vmb,
        }
    }
}
