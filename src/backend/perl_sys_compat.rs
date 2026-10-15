use libperl_sys;

pub use std::ffi::{c_schar, c_void};

pub use libperl_sys::{
    CV,
    OP,
    PERL_EXIT_ABORT,
    PERL_EXIT_DESTRUCT_END,
    PERL_EXIT_EXPECTED,
    PERL_EXIT_WARN,
    // Functions
    PERL_SYS_TERM as Perl_sys_term,
    PL_csighandlerp,
    PL_sig_name,
    PL_sig_num,
    Perl_newXS,
    Perl_rsignal,
    Perl_rsignal_state,
    Perl_sys_init3,
    // Structs
    PerlInterpreter,
    Sighandler_t,
    U8,
    U32,

    perl_alloc,
    perl_construct,
    perl_destruct,
    perl_free,
    perl_parse,
    perl_run,

    XSINIT_t as XsInitFn,
    XSUBADDR_t as XSubFn,
};

// Must be linked and exported with different signature :/
#[inline(always)]
pub unsafe fn Perl_set_context(my_perl: *mut PerlInterpreter) {
    unsafe {
        libperl_sys::perl_core::Perl_set_context(my_perl as *mut c_void);
    }
}

// Not declared in any perl header (defined in the extensions' C files and exported from perl5xx.dll / libperl),
// so bindgen cannot generate them.
// Embedders declare them themselves, like ExtUtils::Embed's xsinit.c.
unsafe extern "C" {
    pub fn boot_DynaLoader(my_perl: *mut PerlInterpreter, cv: *mut CV);

    #[cfg(windows)]
    pub fn boot_Win32CORE(my_perl: *mut PerlInterpreter, cv: *mut CV);
}

#[inline(always)]
pub unsafe fn exit_flags_ptr(my_perl: *mut PerlInterpreter) -> *mut U8 {
    libperl_sys::PL_exit_flags_ptr!(my_perl)
}
#[inline(always)]
pub unsafe fn origalen_ptr(my_perl: *mut PerlInterpreter) -> *mut U32 {
    libperl_sys::PL_origalen_ptr!(my_perl)
}
#[inline(always)]
pub unsafe fn perl_destruct_level_ptr(my_perl: *mut PerlInterpreter) -> *mut c_schar {
    libperl_sys::PL_perl_destruct_level_ptr!(my_perl)
}
#[inline(always)]
pub unsafe fn restartop(my_perl: *mut PerlInterpreter) -> *const OP {
    libperl_sys::PL_restartop!(my_perl).cast_const()
}
