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

# Rust/egui implementation of the C ABI → ffi/target/release/libvidya.so
ffi:
    cargo build --manifest-path ffi/Cargo.toml --release

# Jolt showcase rendered by the Rust/egui backend
jolt-rust: ffi
    cd jolt && LD_LIBRARY_PATH=../ffi/target/release jolt -M:showcase

# Stateful Jolt Control Center on the Rust/egui backend
jolt-rust-app: ffi
    cd jolt && LD_LIBRARY_PATH=../ffi/target/release jolt -M:app

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
