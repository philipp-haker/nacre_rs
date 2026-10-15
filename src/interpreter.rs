use crate::Sys;
use crate::{info, trace};
use crate::default_xs_init;
use super::backend::{
    Perl_set_context, PerlInterpreter, exit_flags_ptr, perl_destruct, perl_free, perl_parse,
    perl_run, XsInitFn,

};
use super::{ExitFlags, New, Parsed, PerlPhase};
use core::ffi::c_int;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::ptr::NonNull;
use std::ffi::OsStr;

pub struct Perl<Phase: PerlPhase> {
    /// Send, !Sync
    _phantom: std::marker::PhantomData<std::cell::UnsafeCell<()>>,
    sys: Sys,
    perl: NonNull<PerlInterpreter>,
    phase: Phase,
}

impl<Phase: PerlPhase> std::fmt::Debug for Perl<Phase> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hides the _phantom and _sys fields
        f.debug_struct("Perl")
            .field("perl", &self.perl)
            .field("phase", &self.phase)
            .finish()
    }
}

impl<Phase: PerlPhase> Drop for Perl<Phase> {
    fn drop(&mut self) {
        info!(
            "Perl interpreter at {:p} no longer in use; destructing; freeing.",
            self.perl.as_ptr()
        );
        let _ = unsafe { self.teardown() };
    }
}

/// General functionality that is NOT EXPOSED
impl<Phase: PerlPhase> Perl<Phase> {
    pub(super) fn from_ext(sys: Sys, perl: NonNull<PerlInterpreter>, phase: Phase) -> Self {
        Self {
            _phantom: PhantomData,
            sys: sys,
            perl,
            phase,
        }
    }

    fn with_phase<P>(self, phase: P) -> Perl<P>
    where
        P: PerlPhase,
    {
        let (sys, perl) = self.into_inner();

        Perl::<P>::from_ext(sys, perl, phase)
    }

    unsafe fn teardown(&mut self) -> i32 {
        self.set_context();
        self.unregister_signal_handlers();
        unsafe {
            let ret = perl_destruct(self.perl.as_ptr());
            perl_free(self.perl.as_ptr());
            ret
        }
    }

    fn into_inner(self) -> (Sys, NonNull<PerlInterpreter>) {
        let mut this = std::mem::ManuallyDrop::new(self);

        let ptr = this.perl;
        let sys = this.sys.clone();
        unsafe {
            std::ptr::drop_in_place(&mut this.sys);
            std::ptr::drop_in_place(&mut this.phase);
        }
        (sys, ptr)
    }

    /// Restores the default action for every signal still routed to perl's C handler,
    /// so no signal reaches a destroyed interpreter (perlmain does this before perl_destruct).
    fn unregister_signal_handlers(&mut self) {
        let my_perl = self.perl.as_ptr();
        // Compare by address: comparing function pointers directly triggers
        // the `unpredictable_function_pointer_comparisons` lint.
        let perl_handler = Sys::c_sighandler().map(|f| f as usize);

        for (_name, sig) in Sys::c_signals() {
            // SAFETY: live interpreter; rsignal(_state) go through its PL_Proc
            // vtable (PERL_IMPLICIT_SYS), so this must run before perl_destruct.
            let current =
                unsafe { super::backend::Perl_rsignal_state(my_perl, sig) }.map(|f| f as usize);
            if current.is_some() && current == perl_handler {
                // None == SIG_DFL
                unsafe { super::backend::Perl_rsignal(my_perl, sig, None) };
            }
        }
    }
}

///Phase independant and public functionality of perl interpreters.
impl<Phase: PerlPhase> Perl<Phase> {
    /// Returns the system used by this perl interpreter.
    /// Usefull if you dropped the system already but need new interpreters.
    ///
    /// You could also call `Sys::initialized().expect("Interpreter still alive")` if you have an iterpreter.
    /// Though an `unwrap` or `expect` does look nasty
    ///
    /// # Returns
    ///
    /// The System that was used to create this interpreter.
    pub fn sys(&self) -> Sys {
        self.sys.clone()
    }

    /// Calls `PERL_SET_CONTEXT` with this interpreter
    pub fn set_context(&mut self) {
        unsafe {
            Perl_set_context(self.perl.as_ptr());
        }
    }

    /// Calls perl_destruct and perl_free, returning the result of `perl_destruct`.
    /// Use this instead of simply dropping, if you need the error code.
    ///
    /// # Returns
    ///
    /// Whatever `perl_destruct` decided to return.
    pub fn destruct(self) -> i32 {
        let mut this = std::mem::ManuallyDrop::new(self);
        unsafe {
            let ret = this.teardown();
            std::ptr::drop_in_place(&mut this.phase);
            std::ptr::drop_in_place(&mut this.sys);
            ret
        }
    }

    /// Returns the exit flags currently associated with this interpreter.
    /// If Multiplicity is enabled these might be unique for each interpreter.
    /// Otherwise they are shared.
    pub fn exit_flags(&self) -> ExitFlags {
        ExitFlags::from_bits(unsafe { *exit_flags_ptr(self.perl.as_ptr()) })
    }

    /// Sets the exitflags to a given value
    pub fn set_exit_flags(&mut self, flags: ExitFlags) {
        unsafe {
            *exit_flags_ptr(self.perl.as_ptr()) = flags.bits();
        }
    }

    /// Lets you modify the exit flags via a closure.
    /// Note that this does not expose the underlying pointer.
    pub fn modify_exit_flags<R>(&mut self, modify: impl FnOnce(&mut ExitFlags) -> R) -> R {
        let mut flags = self.exit_flags();
        let result = modify(&mut flags);
        self.set_exit_flags(flags);
        result
    }

    /// TODO: Add the CV return value ...
    pub fn new_xs(&mut self) {}
}

/// Functionality for new perl interpreters.
impl Perl<New> {

    /// Calls perl_parse with [`default_xs_init`] function, given arguments and no environments.
    /// 
    /// See [`Self::parse`] for more details.
    #[inline]
    pub fn parse_default_no_env<I, S1, S2>(
        self,
        args: &[impl AsRef<OsStr> + Debug],
    ) -> Result<Perl<Parsed>, super::ParseError> {
        Self::parse_default::<[(&'static OsStr, &'static OsStr); 0], _, _>(self, args, None)
    }

    /// Calls `perl_parse` with [`default_xs_init`] as the xs init function.
    /// 
    /// See [`Self::parse`] for more details.
    #[inline]
    pub fn parse_default<I, S1, S2>(
        self,
        args: &[impl AsRef<OsStr> + Debug],
        env: Option<I>,
    ) -> Result<Perl<Parsed>, super::ParseError> 
    where 
        I: IntoIterator<Item = (S1, S2)>,
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr> {
        unsafe {
            Self::parse(self, Some(default_xs_init), args, env)
        }
    }
    
    /// Calls `perl_parse` with the given `xs_init` function and arguments, but no environments.
    /// 
    /// See [`Self::parse`] for more details.
    /// 
    /// # Safety
    /// 
    /// The provided function must be a valid XsInitFn.
    #[inline]
    pub unsafe fn parse_no_env(self, xs_init: XsInitFn, args: &[impl AsRef<OsStr> + Debug]) -> Result<Perl<Parsed>, super::ParseError> {
        unsafe {
            Self::parse::<[(&'static OsStr, &'static OsStr); 0], _, _>(self, xs_init, args, None)
        }
    } 

    #[inline]
    unsafe fn parse<I, S1, S2>(
        self,
        xs_init: XsInitFn,
        args: &[impl AsRef<OsStr> + Debug],
        env: Option<I>,
    ) -> Result<Perl<Parsed>, super::ParseError> 
    where
        I: IntoIterator<Item = (S1, S2)>,
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>
    {
        unsafe {
            Self::_parse(self, xs_init, args, env.map(IntoIterator::into_iter))
        }
    }

    /// Calls `perl_parse`
    /// 
    /// # Safety
    /// 
    /// The provided function must be a valid xs_init Function.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            skip(xs_init, args, env),
            fields(
                xs_init = ?xs_init.map_or(std::ptr::null(), |func| func as *const ()),
                args_len = args.len(),
                env_len = ?env.as_ref().map(|env| env.size_hint())
            )
        )
    )]
    unsafe fn _parse<I, S1, S2>(
        self,
        xs_init: XsInitFn,
        args: &[impl AsRef<OsStr> + Debug],
        env: Option<I>,
    ) -> Result<Perl<Parsed>, super::ParseError> 
    where
        I: Iterator<Item = (S1, S2)>,
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>
    {
        // New phase
        let phase = Parsed::new(args, env);

        // Switch phase
        let parsed_perl = self.with_phase(phase);

        let code = unsafe {
            let raw_code: c_int = perl_parse(
                parsed_perl.perl.as_ptr(),
                xs_init,
                parsed_perl.phase.args().len() as c_int,
                parsed_perl.phase.args().as_mut_ptr(),
                parsed_perl
                    .phase
                    .env()
                    .map_or(std::ptr::null_mut(), |env| env.as_mut_ptr()),
            );
            i32::from(raw_code)
        };
        trace!("perl_parse returned {code}");

        if code == 0 {
            Ok(parsed_perl)
        } else {
            Err(super::ParseError {
                code,
                destruct: parsed_perl.destruct(),
            })
        }
    }

    /// PL_perl_destruct_level. perl_construct sets 1 (MULTIPLICITY); 0 skips the
    /// full cleanup in perl_destruct. $PERL_DESTRUCT_LEVEL can still raise it.
    pub fn set_destruct_level(&mut self, level: i8) {
        // SAFETY: self.perl is a live, constructed interpreter (type invariant).
        unsafe { *super::backend::perl_destruct_level_ptr(self.perl.as_ptr()) = level };
    }

    /// PL_origalen. Non-zero before perl_parse stops perl from computing the argv
    /// area that `$0` assignments may overwrite (perlembed "$0 assignments").
    pub fn set_origalen(&mut self, len: u32) {
        unsafe { *super::backend::origalen_ptr(self.perl.as_ptr()) = len };
    }
}

impl Perl<Parsed> {
    /// Calls `perl_run`, wrapping the `c_int` that is returned.
    /// Is safe to call, as this type internally has a `NonNull<PerlInterpreter>`.
    ///
    /// # Returns
    ///
    /// The return code
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), fields(self = ?self.perl)))]
    pub fn run(&mut self) -> i32 {
        trace!("Running");
        let r = unsafe { i32::from(perl_run(self.perl.as_ptr())) };
        trace!("Returned {r}");
        r
    }

    /// PL_restartop is set while an error from croak is propagating.
    /// perlmain asserts it is unset before perl_run.
    pub fn has_restartop(&self) -> bool {
        !unsafe { super::backend::restartop(self.perl.as_ptr()) }.is_null()
    }
}
