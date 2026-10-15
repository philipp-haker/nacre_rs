//! This module contains some macros for logging/tracing.
//! This allows me to use `trace!`, `debug!`, `info!`, `warn!` and `error!` in code.
//! Those macros then act as a proxy to noops, the log-crate or the tracing-crate.

#[macro_export]
macro_rules! trace {
    ($($t:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            tracing::trace!($($t)*);
            #[cfg(feature = "log")]
            log::error!($($t)*);
            ()
        }
    }
}

#[macro_export]
macro_rules! debug {
    ($($t:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            tracing::debug!($($t)*);
            #[cfg(feature = "log")]
            log::error!($($t)*);
            ()
        }
    }
}

#[macro_export]
macro_rules! info {
    ($($t:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            tracing::info!($($t)*);
            #[cfg(feature = "log")]
            log::error!($($t)*);
            ()
        }
    }
}

#[macro_export]
macro_rules! warn {
    ($($t:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            tracing::warn!($($t)*);
            #[cfg(feature = "log")]
            log::error!($($t)*);
            ()
        }
    }
}

#[macro_export]
macro_rules! error {
    ($($t:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            tracing::error!($($t)*);
            #[cfg(feature = "log")]
            log::error!($($t)*);
            ()
        }
    }
}
