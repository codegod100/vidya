# Vidya for Nim

This directory contains a native Nim implementation of Vidya. The theme,
layout engine, widgets, input handling, and window lifecycle are Nim code;
raylib is used only for platform drawing and input.

The module has two interfaces:

* an idiomatic Nim API for applications that `import vidya`; and
* the existing `vidya_*` C ABI, exported when the module is built as a shared
  library, for Jolt, C, Zig, and other callers.

No Nim raylib package is required. The private `vidya/raylib` module declares
the small raylib ABI surface the implementation needs. A system `libraylib`
must be available when an application starts.

## Run the showcase

```sh
nim c -r --path:nim/src nim/examples/showcase.nim
```

The larger stateful example is `examples/control_center.nim`.

## Build `libvidya`

From this directory:

```sh
nimble build
```

This writes the platform shared library (`build/libvidya.so`,
`build/libvidya.dylib`, or `build/vidya.dll`) and exports the ABI described by
`../raylib/include/vidya.h`.

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

Run `nimble typecheck` to type-check the implementation and examples.
