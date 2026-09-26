# Vidya for Nim

This package provides thin, idiomatic Nim bindings for Vidya's C ABI. The
application owns normal Nim state while Vidya reports control events once per
frame. Both ABI implementations work unchanged:

* [`../ffi`](../ffi/README.md) — Rust/egui, Vidya's native semantic layer.
* [`../raylib`](../raylib/README.md) — the standalone C/raylib implementation.

## Run the showcase

Build a backend, put its directory on the shared-library search path, and add
this package's source directory to Nim's import path:

```sh
cargo build --manifest-path ffi/Cargo.toml --release
nim c -r --path:nim/src nim/examples/showcase.nim
```

On Linux the second command needs `LD_LIBRARY_PATH=ffi/target/release`; on
macOS use `DYLD_LIBRARY_PATH`, and on Windows put `vidya.dll` on `PATH`.

To use the C/raylib backend instead, build it as described in its README and
point the same environment variable at `raylib/build`. No Nim code changes.

The larger stateful example is `examples/control_center.nim`.

## API

```nim
import vidya

var syncing = true

proc render() =
  title("Preferences")
  card:
    body("State stays in the Nim application.")
    discard checkbox("Sync preferences", syncing)
    if primaryButton("Save"):
      echo "saved"

setMode(Mode.dark)
run(render, windowTitle = "My application")
```

`run` supplies the window and frame loop. Lower-level `openWindow`,
`beginFrame`, `pageBegin`, and matching teardown procedures are available when
an application needs to own that loop. The `page` and `card` templates restore
their parent containers even when their bodies raise an exception.

Run `nimble check` in this directory to type-check the module and examples.
