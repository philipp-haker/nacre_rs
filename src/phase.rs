//! Module that contains the two phase a perl interpreter can be in.

use super::FFIStringKeepAlive;
use core::fmt::{Debug, Formatter};
use std::ffi::OsStr;

/// Marker trait to tell the Perl Interpreter in which state it is.
pub trait PerlPhase: Debug {}

/// Marker for a newly created Perl Interpreter.
#[derive(std::fmt::Debug)]
pub struct New;

impl PerlPhase for New {}

/// Type-State for Perl Interpreters that already parsed something
/// A parsed Perl can run code.
pub struct Parsed {
    args: FFIStringKeepAlive,
    env: Option<FFIStringKeepAlive>,
}
impl PerlPhase for Parsed {}
impl Debug for Parsed {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Hides the argv_ptrs and envp_ptrs fields as they are identical to the other ones
        f.debug_struct("Parsed")
            .field("args", &self.args)
            .field("env", &self.env)
            .finish()
    }
}

impl Parsed {
    pub(super) fn new<I, S1, S2>(args: &[impl AsRef<OsStr>], env: Option<I>) -> Self
    where
        I: IntoIterator<Item = (S1, S2)>,
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>,
    {
        use super::env_pair_to_cstring;

        let args = FFIStringKeepAlive::from_os_str(args.iter().map(AsRef::as_ref));
        let env = env.map(|env| {
            FFIStringKeepAlive::from_cstrings(
                env.into_iter()
                    .map(|(k, v)| env_pair_to_cstring(k.as_ref(), v.as_ref())),
            )
        });

        Self { args, env }
    }
    pub(super) fn args(&self) -> &FFIStringKeepAlive {
        &self.args
    }
    pub(crate) fn env(&self) -> Option<&FFIStringKeepAlive> {
        self.env.as_ref()
    }
}
