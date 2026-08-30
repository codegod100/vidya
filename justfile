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
# `reindeer vendor` reflinks out of the cargo cache and fails on some
# filesystems, so the tree is materialized with cargo itself; reindeer only
# reads it. `--locked` matters: an unlocked re-resolve silently drops crates
# that the committed BUCK still references.
vendor:
    cd third-party/rust && cargo vendor --locked --versioned-dirs vendor >/dev/null
    mkdir -p third-party/rust/.cargo
    printf '[source.crates-io]\nreplace-with = "vendored-sources"\n\n[source.vendored-sources]\ndirectory = "vendor"\n' \
        > third-party/rust/.cargo/config.toml
    PATH="{{justfile_directory()}}/scripts:$PATH" reindeer --third-party-dir third-party/rust buckify

# Rust/egui implementation of the C ABI → build/libvidya.so
ffi:
    @just buck build //:libvidya --show-output | awk 'END{print $2}' \
        | xargs -I{} install -Dm755 {} build/libvidya.so

# The same C ABI cross-compiled for a 64-bit Android device, laid out under the
# ABI directory name an APK's lib/ expects. Needs ANDROID_NDK_HOME for the
# linker; everything else (rustc, libstd) is DotSlash-pinned.
ffi-android:
    @just buck build //:libvidya-android --show-output | awk 'END{print $2}' \
        | xargs -I{} install -Dm755 {} build/android/arm64-v8a/libvidya.so

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
