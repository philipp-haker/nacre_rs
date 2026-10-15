// Always present
use core::convert::Infallible;
use core::ffi::{CStr, c_char};
use core::fmt::{Debug, Formatter};

// Not avalable on no-std :(
use std::ffi::{CString, OsStr};

pub mod backend;

// Flattening the project structure from outside
mod sys;
pub use sys::*;

mod interpreter;
pub use interpreter::*;

mod flags;
pub use flags::*;

mod phase;
pub use phase::*;

pub(crate) mod log_trace;

/// This macro extends to the current file as a CStr.
/// Used for the XS stuff.
macro_rules! __FILE__ {
    () => {
        #[allow(unused)]
        unsafe {
            // SAFETY:
            // The bytes are null terminated by the concat! macro
            core::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes())
        }
    };
}

// Checking that exactly one perl_backend is in use;
const _: () = {
    #[cfg(all(feature = "perl_custom", feature = "perl_sys"))]
    compile_error!(r#"Please use either feature flag: "perl_custom" or "perl_sys". Not Both"#);

    #[cfg(not(any(feature = "perl_custom", feature = "perl_sys")))]
    compile_error!(r#"Please use either feature flag: "perl_custom" or "perl_sys"."#);

    ()
};

pub struct ParseError {
    code: i32,
    destruct: i32,
}

impl ParseError {
    pub fn parse_code(&self) -> i32 {
        self.code
    }

    pub fn destruct_code(&self) -> i32 {
        self.destruct
    }
}

/// XS_init
pub unsafe extern "C" fn default_xs_init(my_perl: *mut backend::PerlInterpreter) {
    #[cfg(windows)]
    use backend::boot_Win32CORE;
    use backend::{Perl_newXS, boot_DynaLoader};

    unsafe {
        Perl_newXS(
            my_perl,
            c"DynaLoader::boot_DynaLoader".as_ptr(),
            Some(boot_DynaLoader),
            __FILE__!().as_ptr(),
        );
        #[cfg(windows)]
        let _ = Perl_newXS(
            my_perl,
            c"Win32CORE::bootstrap".as_ptr(),
            Some(boot_Win32CORE),
            __FILE__!().as_ptr(),
        );
    }
}

/// A struct used to keep (C-Style) strings alive for the duration fo some function call.
/// stores as little infromation as possible to ensure little memory overhead
pub struct FFIStringKeepAlive {
    pointers: Vec<*mut c_char>,
}

impl Debug for FFIStringKeepAlive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(
                self.pointers
                    .iter()
                    .copied()
                    .map_while(std::ptr::NonNull::new)
                    .map(|ptr| unsafe { CStr::from_ptr(ptr.as_ptr()) }),
            )
            .finish()
    }
}

impl Drop for FFIStringKeepAlive {
    fn drop(&mut self) {
        for ptr in self.pointers.drain(..) {
            if let Some(non_null) = std::ptr::NonNull::new(ptr) {
                // SAFETY: These pointers are
                // - not null
                // - Created via into CString::into_raw
                let c_string = unsafe { CString::from_raw(non_null.as_ptr()) };
                drop(c_string)
            }
        }
    }
}

impl FFIStringKeepAlive {
    ///
    fn new<I, S, E>(iter: I, convert: impl Fn(S) -> Result<CString, E>) -> Self
    where
        I: IntoIterator<Item = S>,
        E: std::error::Error,
        S: Debug,
    {
        let pointers: Vec<*mut c_char> = iter
            .into_iter()
            .map(|s| (format!("{s:?}"), convert(s)))
            .filter_map(|(dbg, r)| {
                r.inspect_err(|e| {
                    crate::warn!(
                        "Got an error while converting {dbg:?} [{}] into CString: {e}",
                        std::any::type_name::<S>()
                    );
                    ()
                })
                .ok()
            })
            .map(CString::into_raw)
            .chain(std::iter::once(std::ptr::null_mut()))
            .collect();

        Self { pointers }
    }

    #[inline]
    fn new_infallible<I, S>(iter: I, convert: impl Fn(S) -> CString) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Debug,
    {
        Self::new::<I, S, Infallible>(iter, |s| Ok(convert(s)))
    }

    /// Turns an iterator of string
    pub fn from_str_lossy<'a, I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        Self::new_infallible(iter, |s| unsafe {
            CString::from_vec_unchecked(s.bytes().filter(|b| *b != 0).collect())
        })
    }

    pub fn from_os_str<'a, I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a OsStr>,
    {
        Self::new_infallible(iter, os_to_cstring)
    }

    pub fn from_cstrings<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = CString>,
    {
        Self::new_infallible(iter, std::convert::identity)
    }

    pub fn as_ptr(&self) -> *const *const c_char {
        self.pointers.as_ptr() as *const *const c_char
    }

    pub fn as_mut_ptr(&self) -> *mut *mut c_char {
        self.pointers.as_ptr() as *mut *mut c_char
    }

    pub fn len(&self) -> usize {
        self.pointers.len().saturating_sub(1)
    }
}

/// Conversion from OsStr to CString
fn os_to_cstring(s: &OsStr) -> CString {
    #[cfg(unix)]
    let c = CString::new(s.as_bytes()).expect("OS strings contain no NUL"); // byte-exact
    #[cfg(windows)]
    let c = CString::new(s.to_string_lossy().as_bytes()).expect("OS strings contain no NUL"); // lossy, documented
    c
}

fn env_pair_to_cstring(k: &OsStr, v: &OsStr) -> CString {
    #[cfg(unix)]
    let c = CString::new([k.as_bytes(), b"=", v.as_bytes()].concat())
        .expect("OS strings contain no NUL");
    #[cfg(windows)]
    let c = CString::new(format!("{}={}", k.to_string_lossy(), v.to_string_lossy()).into_bytes())
        .expect("OS strings contain no NUL");
    c
}
