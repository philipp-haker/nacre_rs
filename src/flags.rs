use super::backend::{PERL_EXIT_ABORT, PERL_EXIT_DESTRUCT_END, PERL_EXIT_EXPECTED, PERL_EXIT_WARN};

/// A struct representing the perl exit flags.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ExitFlags(u8);

impl ExitFlags {
    pub const EXPECTED: Self = Self::exit_flag(PERL_EXIT_EXPECTED);
    pub const DESTRUCT_END: Self = Self::exit_flag(PERL_EXIT_DESTRUCT_END);
    pub const WARN: Self = Self::exit_flag(PERL_EXIT_WARN);
    pub const ABORT: Self = Self::exit_flag(PERL_EXIT_ABORT);

    const fn exit_flag(value: u32) -> ExitFlags {
        assert!(
            value <= u8::MAX as u32,
            "PERL_EXIT_* flags does not fit PL_exit_flags(U8)."
        );
        ExitFlags(value as u8)
    }

    /// Checks if a certain other flag is contained in this flag
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Turns a [`Self`] into the underlying flag.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Constructs a [`Self`] from the given byte.
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }
}

impl core::fmt::Debug for ExitFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "ExitFlags({:#010b})", self.0)
    }
}

impl core::ops::BitOr for ExitFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl core::ops::BitOrAssign for ExitFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0.bitor_assign(rhs.0);
    }
}
impl core::ops::Not for ExitFlags {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}
impl core::ops::BitAnd for ExitFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl core::ops::BitAndAssign for ExitFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0.bitand_assign(rhs.0);
    }
}
