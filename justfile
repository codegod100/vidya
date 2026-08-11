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

# Gleam UI package (JS target) + example IR fixture
gleam-test:
    cd gleam/vidya && gleam test

gleam-example:
    cd gleam/example && gleam run 2>/dev/null > demo_app.json
    python3 -c "import json; p='gleam/example/demo_app.json'; json.dump(json.load(open(p)), open(p,'w'), indent=2); open(p,'a').write('\n')"
