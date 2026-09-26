version       = "0.1.0"
author        = "Vidya contributors"
description   = "Native Nim implementation of the Vidya UI library"
license       = "MIT"
srcDir        = "src"

requires "nim >= 1.6.0"

task lib, "Build the Nim implementation as libvidya":
  mkDir("build")
  when defined(windows):
    exec "nim c --app:lib --out:build/vidya.dll src/vidya.nim"
  elif defined(macosx):
    exec "nim c --app:lib --out:build/libvidya.dylib src/vidya.nim"
  else:
    exec "nim c --app:lib --out:build/libvidya.so src/vidya.nim"

task typecheck, "Type-check the library and examples":
  exec "nim check src/vidya.nim"
  exec "nim check --path:src examples/showcase.nim"
  exec "nim check --path:src examples/control_center.nim"
