# gleam_fib

Minimal Gleam package (`target = "wasm"`) compiled by the Vidya desktop host’s
`build.rs` and invoked via wasmtime as `gleam_fib__fib`.

Requires a wasm-capable Gleam binary (branch `wasm` of
[nandi.uk/gleam](https://tangled.org/nandi.uk/gleam)):

```bash
export GLEAM=/path/to/gleam/target/debug/gleam
just fib          # build + call fib(10), no GUI
just host         # same guest, then open the showcase
```
