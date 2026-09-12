use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct VarProtectionFlags: i32 {
        const READ     = 1 << 0;
        const WRITE    = 1 << 1;
        const EXECUTE  = 1 << 2;
    }
}

bitflags! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct VarFlags: i32 {
        const ANON     = 1 << 0;
    }
}

pub struct Var {
    pub base_address: u64,
    pub length: usize,
    pub protections: VarProtectionFlags,
    pub flags: VarFlags,
}

impl Var {
    pub fn new(base: u64, length: usize, protections: VarProtectionFlags, flags: VarFlags) -> Self {
        Self {
            base_address: base,
            length,
            protections,
            flags,
        }
    }
}
