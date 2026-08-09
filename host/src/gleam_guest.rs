//! Load the Gleam-compiled Wasm guest and call `gleam_fib__fib`.

use wasmtime::{Engine, Instance, Module, Store};

const GUEST_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gleam_fib.wasm"));

/// Linked export name from `examples/gleam_fib` (`module__function`).
const FIB_EXPORT: &str = "gleam_fib__fib";

pub fn fib(n: i64) -> Result<i64, String> {
    let engine = Engine::default();
    let module = Module::new(&engine, GUEST_WASM)
        .map_err(|e| format!("parse gleam_fib.wasm: {e}"))?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])
        .map_err(|e| format!("instantiate gleam guest: {e}"))?;
    let fib = instance
        .get_typed_func::<i64, i64>(&mut store, FIB_EXPORT)
        .map_err(|e| format!("export {FIB_EXPORT}: {e}"))?;
    fib.call(&mut store, n)
        .map_err(|e| format!("call {FIB_EXPORT}({n}): {e}"))
}
