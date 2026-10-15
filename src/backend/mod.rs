//! Small redirect module that just forwards all defined functions etc.
//! This way every piece of code can reference the backend, and the only switching happens here

#[cfg(feature = "perl_custom")]
mod perl_custom;
#[cfg(feature = "perl_custom")]
pub use self::perl_custom::*;

#[cfg(feature = "perl_sys")]
mod perl_sys_compat;
#[cfg(feature = "perl_sys")]
pub use self::perl_sys_compat::*;
