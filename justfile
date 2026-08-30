# Vidya
#   nix develop
#   just waydroid
#   just host

set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# Desktop window
host *args:
    cargo run --manifest-path host/Cargo.toml {{args}}

lib:
    cargo build --lib

# buck2, with the DotSlash-pinned tools (rustc, zig) on PATH.
buck *args:
    PATH="{{justfile_directory()}}/scripts:$PATH" ./scripts/buck2 {{args}}

# Re-vendor third-party crates and regenerate third-party/rust/BUCK.
# Needed after changing third-party/rust/Cargo.toml, and on a fresh clone —
# the vendor tree is generated, not committed.
vendor:
    PATH="{{justfile_directory()}}/scripts:$PATH" reindeer --third-party-dir third-party/rust vendor
    PATH="{{justfile_directory()}}/scripts:$PATH" reindeer --third-party-dir third-party/rust buckify

# Rust/egui implementation of the C ABI → build/libvidya.so
ffi:
    @just buck build //:libvidya --show-output | awk 'END{print $2}' \
        | xargs -I{} install -Dm755 {} build/libvidya.so

# Jolt showcase rendered by the Rust/egui backend
jolt-rust: ffi
    cd jolt && LD_LIBRARY_PATH=../build ../scripts/jolt -M:showcase

# Stateful Jolt Control Center on the Rust/egui backend
jolt-rust-app: ffi
    cd jolt && LD_LIBRARY_PATH=../build ../scripts/jolt -M:app

# APK in android-demo/ → Waydroid
waydroid:
    ./scripts/waydroid-demo.sh run

run: waydroid

install:
    ./scripts/waydroid-demo.sh install

launch:
    ./scripts/waydroid-demo.sh launch

shots:
    ./scripts/waydroid-demo.sh shots

# ARM64 NativeActivity APK (cimgui + raylib) for a connected physical device.
android-native:
    bash raylib/android/build-apk.sh build

android-native-run:
    bash raylib/android/build-apk.sh run
