use core::future::Future;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

/// Simple macro for brainstorming API designs.
#[macro_export]
macro_rules! api_planning {
    ($($tt:tt)*) => {};
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

/// Returns true if running in the browser, or an environment supporting
/// browser operations.
pub fn is_browser() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    return false;

    #[cfg(target_arch = "wasm32")]
    {
        // Memoize the value
        static mut IS_BROWSER: Option<bool> = None;

        if let Some(is_browser) = unsafe { IS_BROWSER } {
            is_browser
        } else {
            // We need to manually check. We can't use the `web_sys::window` fn
            // because it relies on the window already existing...ironically...
            let global_this = js_sys::global();

            let window =
                js_sys::Reflect::get(&global_this, &"window".into()).unwrap();

            if window.is_undefined() {
                unsafe { IS_BROWSER = Some(false) };

                return false;
            }

            let document =
                js_sys::Reflect::get(&window, &"document".into()).unwrap();

            let is_browser = !document.is_undefined();

            unsafe { IS_BROWSER = Some(is_browser) };

            is_browser
        }
    }
}
