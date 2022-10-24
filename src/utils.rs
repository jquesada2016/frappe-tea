/// Helper macro for planning out new API designs.
///
/// This macro just consumes all input and does nothing with it.
#[allow(unused_macros)]
macro_rules! api_planning {
  ($($tt:tt)*) => {};
}

macro_rules! trace {
  ($($tt:tt)*) => {
    #[cfg(debug_assertions)]
    tracing::debug!($($tt)*);
  };
}

#[allow(unused_macros)]
macro_rules! debug {
  ($($tt:tt)*) => {
    #[cfg(debug_assertions)]
    tracing::debug!($($tt)*);
  };
}
