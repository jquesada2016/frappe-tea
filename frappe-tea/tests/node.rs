//! Test suite for Node.js

#![cfg(target_arch = "wasm32")]
#![cfg(feature = "node-tests")]

extern crate wasm_bindgen_test;
use frappe_tea::prelude::*;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn is_not_browser() {
    assert!(!env::is_browser());
}
