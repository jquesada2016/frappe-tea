#![feature(once_cell)]

#[macro_use]
extern crate async_trait;
#[allow(unused_imports)]
#[macro_use]
extern crate static_assertions;

#[macro_use]
mod utils;
mod non_wasm;
pub mod testing;
mod wasm;

pub mod prelude {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    pub use non_wasm::*;
    #[cfg(target_arch = "wasm32")]
    pub use wasm::*;
}
