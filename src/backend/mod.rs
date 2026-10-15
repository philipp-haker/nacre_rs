//! Small redirect module that just forwards all defined functions etc.
//! This way every piece of code can reference the backend, and the only switching happens here

// Checking that at least one perl_backend is in use
// If both are enabled, then perl_sys takes precedence
const _: () = {
    #[cfg(not(any(feature = "perl_custom", feature = "perl_sys")))]
    compile_error!(r#"Please use either feature flag: "perl_custom" or "perl_sys"."#);

    ()
};

#[cfg(feature = "perl_custom")]
#[cfg_attr(feature = "perl_sys", allow(dead_code, unused_imports))]
mod perl_custom;
#[cfg(all(feature = "perl_custom", not(feature = "perl_sys")))]
pub use self::perl_custom::*;

#[cfg(feature = "perl_sys")]
mod perl_sys_compat;
#[cfg(all(feature = "perl_sys"))]
pub use self::perl_sys_compat::*;
