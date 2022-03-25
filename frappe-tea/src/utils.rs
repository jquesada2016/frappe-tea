use core::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use futures::executor::LocalPool;
use futures::task::{LocalSpawn, LocalSpawnExt, SpawnExt};
use std::{lazy::SyncOnceCell, ops::Deref, sync::Arc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[macro_export]
macro_rules! api_planning {
    ($($tt:tt)*) => {};
}

pub async fn execute_async<Fut>(future: Fut)
where
    Fut: Future<Output = ()> + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        let tp = LocalPool::new();

        tp.spawner().spawn_local(future);
    }

    #[cfg(target_arch = "wasm32")]
    spawn_local(future);
}
