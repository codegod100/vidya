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

# ARM64 NativeActivity APK (cimgui + raylib) for a connected physical device.
android-native:
    bash raylib/android/build-apk.sh build

android-native-run:
    bash raylib/android/build-apk.sh run
