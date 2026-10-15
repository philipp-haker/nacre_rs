//! This module contains the drop guard to terminte the perl system when no interpreter is in use anymore.
//!
//! [`perl_sys::Sys`] is the wrapper type that is exposed to the user.
//! It holds an Arc containing the drop-guard that terminates the perl system.
//!
//! As long as one system is active, no new one will be created.
//!
//! Use [`perl_sys::Sys::init3`] to create a new system.
//! If a system is already active, you will NOT get a new one, even if you pass new arguments.
//! You will get the running system.
//!
//! Use [`Sys::get_initialized`] to get the already running system, if one exists.
//!
use super::backend::{Perl_sys_init3, Perl_sys_term, perl_alloc, perl_construct};
use super::{FFIStringKeepAlive, New, Perl};
use crate::{debug, info, trace, warn};
use core::ffi::{CStr, c_char, c_int};
use core::fmt::{Debug, Formatter};
use core::ptr::NonNull;
use std::ffi::OsStr;
use std::sync::{Arc, Mutex, Weak};

/// A global SYS_GUARD to avoid creating new PerlSys Instances if that is not neccessary.
/// As long as this weak reference can be upgraded, new calls to [`PerlSys::init3`] will NOT call `perl_sys_init3`!
static SYS_GUARD: Mutex<Weak<PerlSysInner>> = Mutex::new(Weak::new());

/// A guard that calls `perl_sys_term` on drop.
struct PerlSysInner {
    _private: [u8; 0],
}

impl Drop for PerlSysInner {
    fn drop(&mut self) {
        info!("perl_sys no longer in use; terminating.");
        unsafe {
            Perl_sys_term();
        }
    }
}

impl PerlSysInner {
    /// Wraps `perl_sys_init3` by creating c-compatible arguments and storing themfor the duration of the call.
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    fn _init3<S1, S2>(
        args: &[impl AsRef<OsStr> + Debug],
        env: impl IntoIterator<Item = (S1, S2)> + Debug,
    ) -> Self
    where
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>,
    {
        let mut argc = args.len() as c_int;

        let args = FFIStringKeepAlive::from_os_str(args.iter().map(AsRef::as_ref));
        let env = FFIStringKeepAlive::from_cstrings(
            env.into_iter()
                .map(|(k, v)| super::env_pair_to_cstring(k.as_ref(), v.as_ref())),
        );

        let mut args_ptr = args.as_mut_ptr();
        let mut env_ptr = env.as_mut_ptr();

        debug!("Calling Perl_sys_init3");
        unsafe {
            Perl_sys_init3(
                std::ptr::from_mut(&mut argc),
                std::ptr::from_mut(&mut args_ptr),
                std::ptr::from_mut(&mut env_ptr),
            )
        }
        Self { _private: [] }
    }
}

/// A lifecycle managing struct.
/// Creating one will call `perl_sys_init3`.
/// If no more instances are alive `perl_sys_term` is called.
/// Keeps itself alive as long as an interpreter is alive.
///
/// Is the only way to create perl interpreters.
/// Sys is a singleton.
#[derive(Clone)]
pub struct Sys {
    /// Shared ownership of the perl-sys guard.
    /// Every sys and interpreter keeps an Arc to the same struct.
    inner: Arc<PerlSysInner>,
}
impl PartialEq for Sys {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}
impl Eq for Sys {}

impl Debug for Sys {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // SAFETY
        // Only used to get nice looking debugging output.
        // Neither read nor written to
        let ptr = unsafe { NonNull::new_unchecked(Arc::as_ptr(&self.inner) as *mut PerlSysInner) };

        f.debug_struct("Sys").field("id", &ptr).finish()
    }
}

impl Sys {
    /// Returns a [`Self`] if one has been initialized and is still active.
    /// If none is returned you have to initialize one via [`Self::init3`].
    #[allow(unused)]
    pub fn get_initialized() -> Option<Self> {
        match SYS_GUARD.lock() {
            Ok(weak) => weak.upgrade().map(|inner| Self { inner }),
            Err(_e) => None,
        }
    }

    /// Checks if a [`PerlSys`] is currently in use.
    /// If not `perl_sys_init3` will be used to initialize it.
    /// Otherwise that existing system will be returned.
    ///
    /// # Parameters
    /// - `args`
    /// - `env`
    ///
    /// # Returns
    /// - `(perl_sys, true)` if a NEW [`Sys`] was initialized and the provided arguments were used to initilize it.
    /// - `(perl_sys, false)` if an ALREADY EXISTING [`Sys`] was returned and the provided arguments were therefore ignored.
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn init3<S1, S2>(
        args: &[impl AsRef<OsStr> + Debug],
        env: impl IntoIterator<Item = (S1, S2)> + Debug,
    ) -> (Self, bool)
    where
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>,
    {
        let (inner, flag) = match SYS_GUARD.lock() {
            Ok(mut weak) => {
                if let Some(guard) = weak.upgrade() {
                    debug!("Found active perl_sys; resuing");
                    (guard, false)
                } else {
                    info!("No perl_sys is active; creating new one");
                    let guard = Arc::new(PerlSysInner::_init3(args, env));
                    *weak = Arc::downgrade(&guard);
                    (guard, true)
                }
            }
            Err(mut e) => {
                warn!("SYS_GUARD was poisoned; creating new perl_sys");
                let guard = Arc::new(PerlSysInner::_init3(args, env));
                **e.get_mut() = Arc::downgrade(&guard);
                SYS_GUARD.clear_poison();
                info!("Cleared poison");
                (guard, true)
            }
        };

        (Sys { inner }, flag)
    }

    /// Creates a new perl instance by calling `perl_alloc` and `perl_construct` (in that order).
    /// Return None if `perl_alloc` return a null pointer.
    pub fn new_perl(&self) -> Option<Perl<New>> {
        trace!("Calling perl_alloc");
        let perl = unsafe { NonNull::new(perl_alloc())? };

        trace!("Calling perl_construct");
        unsafe {
            perl_construct(perl.as_ptr());
        };

        trace!("Constructed interpreter at {:p}", perl.as_ptr());
        Some(super::Perl::from_ext(self.clone(), perl, New))
    }

    /// The C-level handler perl installs for %SIG entries (PL_csighandlerp).
    /// Process-global and mutable, so only the current value is meaningful.
    pub(crate) fn c_sighandler() -> super::backend::Sighandler_t {
        unsafe { super::backend::PL_csighandlerp }
    }

    /// Returns an iterator over all the signals.
    /// These are the name and number, in their c-style representation.
    ///
    /// If you want to work in rust, then [`Self::signals`] is probably what you want.
    pub fn c_signals() -> CSignals {
        // signal 0 is a placeholder
        CSignals { current_index: 1 }
    }

    /// Returns an iterator over all the signals that have utf8 names.
    /// These are the name and number of the signal.
    ///
    /// If you want to work with the Perl-C-API directly, you probably want [`Self::c_signals`]
    pub fn signals() -> Signals {
        Signals {
            c_signals: Self::c_signals(),
        }
    }
}

/// An iterator over the signal names and corresponding numbers.
/// These are the c-representations.
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct CSignals {
    current_index: usize,
}

impl CSignals {
    fn names() -> *const *const c_char {
        (&raw const super::backend::PL_sig_name).cast::<*const c_char>()
    }

    fn nums() -> *const c_int {
        (&raw const super::backend::PL_sig_num).cast::<c_int>()
    }
}

impl std::iter::Iterator for CSignals {
    type Item = (&'static CStr, c_int);
    fn next(&mut self) -> Option<Self::Item> {
        let name: *const c_char = unsafe { *Self::names().add(self.current_index) };
        let num: c_int = unsafe { *Self::nums().add(self.current_index) };

        if name.is_null() {
            return None;
        }

        self.current_index += 1;

        Some((unsafe { CStr::from_ptr(name) }, num))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut remaining = 0;

        let names = Self::names();

        while !unsafe { { *names.add(self.current_index + remaining) }.is_null() } {
            remaining += 1;
        }

        return (remaining, Some(remaining));
    }
}

impl std::iter::ExactSizeIterator for CSignals {}
impl std::iter::FusedIterator for CSignals {}

/// Wrapper around [`CSignals`] that returns all the valid utf8-named signals.
/// It drops all signals that are not utf8, though that should not happen.
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Signals {
    c_signals: CSignals,
}

impl std::iter::Iterator for Signals {
    type Item = (&'static str, i32);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((c_str, num)) = self.c_signals.next() {
            if let Ok(str) = CStr::to_str(c_str) {
                return Some((str, num as i32));
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        return (0, self.c_signals.size_hint().1);
    }
}

impl std::iter::FusedIterator for Signals {}
