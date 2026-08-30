# Vidya for raylib

This directory is the C implementation of Vidya's semantic UI layer. Its default
backend uses cimgui/Dear ImGui for widgets and `rlImGui` for raylib integration.
It exposes a small C99 ABI suitable for Zig, Jolt, and other FFI consumers.

The API is immediate-mode: call `vidya_begin_frame`, submit the page and its
controls in visual order, then call `vidya_end_frame`. Application state remains
owned by the caller.

The cimgui backend prefers static Ubuntu fonts and falls back to DejaVu Sans. It
uses separate regular and bold fonts, and Dear ImGui owns rasterization,
anti-aliasing, focus, clipping, and control interaction.

## Build

With an installed raylib:

```sh
cmake -S raylib -B raylib/build
cmake --build raylib/build
./raylib/build/vidya-showcase
```

Or let CMake fetch a pinned raylib release:

```sh
cmake -S raylib -B raylib/build -DVIDYA_FETCH_RAYLIB=ON
cmake --build raylib/build
```

The backend can be selected explicitly:

```sh
# Recommended: Dear ImGui widgets through cimgui, raylib platform/rendering
cmake -S raylib -B raylib/build-cimgui \
  -DVIDYA_FETCH_RAYLIB=ON -DVIDYA_BACKEND=cimgui

# Original dependency-light renderer made directly from raylib primitives
cmake -S raylib -B raylib/build-direct \
  -DVIDYA_FETCH_RAYLIB=ON -DVIDYA_BACKEND=direct
```

Zig can provide both compilers on a machine without a system C toolchain:

```sh
cmake -S raylib -B raylib/build-cimgui -G Ninja \
  -DCMAKE_C_COMPILER="$(command -v zig)" -DCMAKE_C_COMPILER_ARG1=cc \
  -DCMAKE_CXX_COMPILER="$(command -v zig)" -DCMAKE_CXX_COMPILER_ARG1=c++ \
  -DVIDYA_FETCH_RAYLIB=ON -DVIDYA_BACKEND=cimgui
cmake --build raylib/build-cimgui
```

The C++ setting is required only while configuring raylib's upstream CMake
project; Vidya and raylib themselves are built as C.

## ABI policy

The exported interface uses only C integers, floats, pointers, and UTF-8 byte
strings. No raylib structs cross the boundary. UI calls and the window lifecycle
must stay on the thread that called `vidya_open`.
