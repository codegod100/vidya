# Vidya for raylib

This directory is the C implementation of Vidya's semantic UI layer. Its default
backend uses cimgui/Dear ImGui for widgets and `rlImGui` for raylib integration.
It exposes a small C99 ABI suitable for Zig, Jolt, and other FFI consumers.

The API is immediate-mode: call `vidya_begin_frame`, submit the page and its
controls in visual order, then call `vidya_end_frame`. Application state remains
owned by the caller.

Dear ImGui owns rasterization, anti-aliasing, focus, clipping, and control
interaction.

## Text

The cimgui backend picks one interface family for both weights, so headings
never switch typeface. It prefers Ubuntu, the family the egui implementation
ships, and then searches GNOME's fonts, the common distribution defaults, and
the system fonts of macOS, Windows, and Android. Families that install a single
variable file, as Adwaita Sans and Cantarell do, have their bold named instance
selected rather than being synthetically emboldened.

The DejaVu subset in `assets/vidya-symbols.ttf` is embedded in the library and
merged behind the interface font, so arrows, bullets, curly quotes, box drawing,
and math-in-prose keep rendering when that font has no glyph for them. This
matches `src/fonts.rs` in the egui implementation.

Glyphs are rasterized by FreeType with light hinting, which is what desktop
toolkits use for interface text. Configure with `-DVIDYA_FREETYPE=OFF`, or build
where FreeType is absent, and cimgui keeps Dear ImGui's bundled stb_truetype;
that build also skips single-weight families, because stb_truetype offers
neither variable instances nor a synthetic bold.

Windows are created with `FLAG_WINDOW_HIGHDPI`. rlImGui reports the resulting
scale as `io.DisplayFramebufferScale`, which Dear ImGui bakes glyphs at, so
HiDPI text is rasterized at the physical resolution instead of magnified.
Layout stays in logical units.

`vidya_load_font` replaces the interface font at runtime. Dear ImGui bakes each
size on demand, so its `atlas_size` argument — a rasterization hint for the
direct backend's fixed atlas — is ignored here.

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

## Android ARM64

The Android build targets a physical `arm64-v8a` device. It cross-compiles
`vidya.examples.control-center` to an ARM64 Chez boot image and embeds the Jolt
application, Chez runtime, Vidya, cimgui, and raylib in one NativeActivity:

```sh
raylib/android/build-apk.sh build
raylib/android/build-apk.sh run
```

The script uses the SDK at `~/.local/share/android-sdk` and NDK r29 at
`~/.local/share/android-ndk-r29` by default. Override `ANDROID_HOME` or
`ANDROID_NDK_HOME` when they are installed elsewhere. It expects `jolt` on
`PATH` and a PIC-enabled Chez `tarm64le` cross build at
`~/.cache/vidya-chez-android`; override those with `JOLT` and `CHEZ_ANDROID`.

Jolt's generated launcher is compiled with `JOLT_NO_FLAT_SPLIT=1`, because the
runtime and application must form one boot image on Android. The NativeActivity
registers every Vidya C function explicitly with Chez rather than relying on
Android's dynamic loader to discover symbols in `libmain.so`.

## ABI policy

The exported interface uses only C integers, floats, pointers, and UTF-8 byte
strings. No raylib structs cross the boundary. UI calls and the window lifecycle
must stay on the thread that called `vidya_open`.
