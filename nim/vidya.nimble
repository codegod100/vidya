version       = "0.1.0"
author        = "Vidya contributors"
description   = "Native Nim implementation of the Vidya UI library"
license       = "MIT"
srcDir        = "src"
binDir        = "build"
namedBin["vidya"] = when defined(windows): "vidya.dll" elif defined(macosx): "libvidya.dylib" else: "libvidya.so"

requires "nim >= 1.6.0"

task typecheck, "Type-check the library and examples":
  exec "nim check src/vidya.nim"
  exec "nim check --path:src examples/showcase.nim"
  exec "nim check --path:src examples/control_center.nim"
