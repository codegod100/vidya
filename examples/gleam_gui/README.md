# gleam_gui

Minimal Gleam package (`target = "wasm"`) — pure **integer calculator**
(`new` / `digit` / `op` / `equals` / `clear` / `clear_entry` / `display` /
`pending_op` / `errored`) — compiled by the Vidya desktop host’s `build.rs`
and invoked via wasmtime. Overview renders the keypad; Gleam owns the packed
model updates.

Requires a wasm-capable Gleam binary (branch `wasm` of
[nandi.uk/gleam](https://tangled.org/nandi.uk/gleam)):

```bash
export GLEAM=/path/to/gleam/target/debug/gleam
just gleam-gui    # build + multi-step calc smoke, no window
just host         # same guests, then open the showcase
```

## Ops

`op` codes: `1=+`, `2=−`, `3=×`, `4=÷`. Values stay in `0..999_999`; overflow /
divide-by-zero sets the error flag (display shows “Error” in the host UI).

## Model

```
model = ((((error * 2 + fresh) * 8 + op) * 1_000_000 + acc) * 1_000_000 + entry
```
