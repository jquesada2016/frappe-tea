use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/browser-or-node.js")]
extern "C" {
    #[wasm_bindgen(js_name = "isBrowser")]
    pub static IS_BROWSER: bool;

    #[wasm_bindgen(js_name = "isNode")]
    pub static IS_NODE: bool;

    #[wasm_bindgen(js_name = "isWebWorker")]
    pub static IS_WEB_WORKER: bool;

    #[wasm_bindgen(js_name = "isJsDom")]
    pub static IS_JS_DOM: bool;

    #[wasm_bindgen(js_name = "isDeno")]
    pub static IS_DENO: bool;

}
