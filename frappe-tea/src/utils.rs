use core::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use futures::executor::block_on;
use std::ops::Deref;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

use crate::bindings;

#[macro_export]
macro_rules! api_planning {
    ($($tt:tt)*) => {};
}

pub fn execute_async<Fut>(future: Fut)
where
    Fut: Future<Output = ()> + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    block_on(future);

    #[cfg(target_arch = "wasm32")]
    spawn_local(future);
}
