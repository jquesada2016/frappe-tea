use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub type Process;

    #[wasm_bindgen(js_namespace = global, catch)]
    pub fn process() -> Result<Process, JsValue>;
}
