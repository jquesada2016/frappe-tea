use core::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use futures::executor::ThreadPool;
use futures::task::SpawnExt;
use std::{lazy::SyncOnceCell, ops::Deref, sync::Arc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[macro_export]
macro_rules! api_planning {
    ($($tt:tt)*) => {};
}

pub async fn execute_async<Fut>(future: Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        static THREAD_POOL: SyncOnceCell<ThreadPool> = SyncOnceCell::new();

        let tp = THREAD_POOL.get_or_init(|| {
            ThreadPool::new().expect("failed to create thread pool")
        });

        tp.spawn(future);
    }

    #[cfg(target_arch = "wasm32")]
    spawn_local(future);
}
