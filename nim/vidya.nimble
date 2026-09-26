version       = "0.1.0"
author        = "Vidya contributors"
description   = "Idiomatic Nim bindings for the Vidya C ABI"
license       = "MIT"
srcDir        = "src"

requires "nim >= 1.6.0"

task check, "Type-check the bindings and examples":
  exec "nim check src/vidya.nim"
  exec "nim check --path:src examples/showcase.nim"
  exec "nim check --path:src examples/control_center.nim"
