# Vidya
#   nix develop
#   just waydroid
#   just host

set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# Desktop window (also builds + loads examples/gleam_fib via wasmtime)
host *args:
    cargo run --manifest-path host/Cargo.toml {{args}}

# Build Gleam → Wasm guest and call fib(10) (no GUI)
fib:
    cargo run --manifest-path host/Cargo.toml -- --fib-only

lib:
    cargo build --lib

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
