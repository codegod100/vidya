//// Pure `fib` for the Vidya desktop host.
////
//// Built by `host/build.rs` via the wasm-capable Gleam compiler
//// (`GLEAM` or sibling `~/code/gleam`). Export name after linking:
//// `gleam_fib__fib`.

pub fn fib(n: Int) -> Int {
  case n {
    0 -> 0
    1 -> 1
    _ -> fib(n - 1) + fib(n - 2)
  }
}
