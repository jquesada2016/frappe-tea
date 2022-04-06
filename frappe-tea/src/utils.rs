use core::future::Future;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[macro_export]
macro_rules! api_planning {
    ($($tt:tt)*) => {};
}

#[macro_export]
macro_rules! cloned {
    () => {};
    ([$($tt:tt)*], $expr:expr) => {{
        cloned!($($tt)*);

        $expr
    }};
    ($(,)? mut { $expr:expr } as $ident:ident $($tt:tt)*) => {
        let mut $ident = ::core::clone::Clone::clone(&$expr);
        cloned!($($tt)*);
    };
    ($(,)? mut $ident:ident $($tt:tt)*) => {
        let mut $ident = ::core::clone::Clone::clone(&$ident);
        cloned!($($tt)*);
    };
    ($(,)? { $expr:expr } as $ident:ident $($tt:tt)*) => {
        let $ident = ::core::clone::Clone::clone(&$expr);
        cloned!($($tt)*);
    };
    ($(,)? $ident:ident $($tt:tt)*) => {
        let $ident = ::core::clone::Clone::clone(&$ident);
        cloned!($($tt)*);
    };
    ($(,)?) => {};
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<Fut>(future: Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn<Fut>(future: Fut)
where
    Fut: Future<Output = ()> + 'static,
{
    spawn_local(future);
}
