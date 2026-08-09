# Vidya
#   nix develop   # puts rustup/cargo on PATH (+ optional RUSTFLAGS)
#   just gleam-app  # whole-window Gleam mini-app (thin Vidya shell)
#   just gleam-str  # string ABI smoke (host↔guest)
#   just host       # aesthetic showcase
#   just fib / just gleam-gui / just gleam-shell / just waydroid

set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# Whole-window Gleam app (Gleam owns UI tree; Vidya themes + paints opcodes)
gleam-app *args:
    cargo run --manifest-path host/Cargo.toml -- --gleam-app {{args}}

# Aesthetic showcase desktop window (also builds + loads Gleam Wasm guests)
# Requires cargo on PATH — enter via `nix develop` if rustup proxies are missing.
host *args:
    cargo run --manifest-path host/Cargo.toml {{args}}

# Alias: same as `just host`
showcase *args:
    cargo run --manifest-path host/Cargo.toml -- --showcase {{args}}

# Build Gleam → Wasm fib guest and call fib(10) (no GUI)
fib:
    cargo run --manifest-path host/Cargo.toml -- --fib-only

# Build Gleam → Wasm calculator guest and smoke multi-step turns (no GUI)
gleam-gui:
    cargo run --manifest-path host/Cargo.toml -- --gui-only

# Build Gleam → Wasm TEA shell guest and smoke Home/About + Inc/Dec/Reset (no GUI)
gleam-shell:
    cargo run --manifest-path host/Cargo.toml -- --shell-only

# Build Gleam → Wasm string guest and smoke read/write/concat/eq (no GUI)
gleam-str:
    cargo run --manifest-path host/Cargo.toml -- --str-only

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
