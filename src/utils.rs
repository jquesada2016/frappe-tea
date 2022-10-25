use futures::Future;

use crate::prelude::Ctx;

/// Helper macro for planning out new API designs.
///
/// This macro just consumes all input and does nothing with it.
#[allow(unused_macros)]
macro_rules! api_planning {
  ($($tt:tt)*) => {};
}

#[allow(unused_macros)]
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

/// Spawns a `!Send` [`Future`].
pub fn spawn_local(fut: impl Future<Output = ()> + 'static) {
  cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
      wasm_bindgen_futures::spawn_local(fut);
    } else if #[cfg(feature = "tokio")] {
      let local = tokio::task::LocalSet::new();

      local.spawn_local(fut);
    }
  }
}

/// Spawns a `Send` [`Future`].
pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
  cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
      wasm_bindgen_futures::spawn_local(fut);
    } else if #[cfg(feature = "tokio")] {
      tokio::task::spawn(fut);
    }
  }
}
