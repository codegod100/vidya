# Vidya for Jolt

These are thin, idiomatic Jolt wrappers around the C ABI in `../raylib`.

```sh
cmake -S raylib -B raylib/build -G Ninja \
  -DCMAKE_C_COMPILER="$(command -v zig)" -DCMAKE_C_COMPILER_ARG1=cc \
  -DCMAKE_CXX_COMPILER="$(command -v zig)" -DCMAKE_CXX_COMPILER_ARG1=c++ \
  -DVIDYA_FETCH_RAYLIB=ON
cmake --build raylib/build
cd jolt
LD_LIBRARY_PATH=../raylib/build jolt -M:app
```

On macOS, use `DYLD_LIBRARY_PATH=../raylib/build`.

`jolt -M:app` runs the stateful Control Center example. It demonstrates
cross-frame Jolt atoms, conditional controls, theme switching, settings,
connection state, and save/reset actions. `jolt -M:showcase` runs the smaller
widget showcase.

State belongs to the Jolt application. Controls report events during the frame;
the caller updates atoms or other application state and redraws it next frame.
The C layer never retains Jolt strings or callbacks.
