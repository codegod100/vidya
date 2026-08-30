#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RAYLIB="$ROOT/raylib"
ANDROID_HOME="${ANDROID_HOME:-$HOME/.local/share/android-sdk}"
ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$HOME/.local/share/android-ndk-r29}"
BUILD="$RAYLIB/android/build"
NATIVE="$BUILD/native"
JOLT_BUILD="$BUILD/jolt"
STAGE="$BUILD/stage"
TOOLS="$ANDROID_HOME/build-tools/36.0.0"
ADB="${ADB:-$ANDROID_HOME/platform-tools/adb}"
PACKAGE="uk.nandi.vidya.native"
ACTIVITY="$PACKAGE/android.app.NativeActivity"

for path in \
  "$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" \
  "$ANDROID_HOME/platforms/android-36/android.jar" \
  "$TOOLS/aapt2" "$TOOLS/zipalign" "$TOOLS/apksigner"; do
  [[ -e "$path" ]] || { echo "missing Android tool: $path" >&2; exit 1; }
done

"$RAYLIB/android/build-jolt-boot.sh" "$JOLT_BUILD"
(
  cd "$JOLT_BUILD"
  "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-objcopy" \
    --input-target=binary \
    --output-target=elf64-littleaarch64 \
    --binary-architecture=aarch64 \
    jolt.boot jolt_boot.o
)

cmake -S "$RAYLIB" -B "$NATIVE" -G Ninja \
  -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" \
  -DANDROID_ABI=arm64-v8a \
  -DANDROID_PLATFORM=android-28 \
  -DPLATFORM=Android \
  -DVIDYA_FETCH_RAYLIB=ON \
  -DVIDYA_BACKEND=cimgui \
  -DVIDYA_BUILD_EXAMPLE=OFF \
  -DVIDYA_BUILD_ANDROID_APP=ON \
  -DVIDYA_JOLT_BOOT_OBJECT="$JOLT_BUILD/jolt_boot.o" \
  -DVIDYA_CHEZ_ANDROID="${CHEZ_ANDROID:-$HOME/.cache/vidya-chez-android}" \
  -DCMAKE_BUILD_TYPE=Release
cmake --build "$NATIVE" --target main

rm -rf "$STAGE"
mkdir -p "$STAGE/lib/arm64-v8a"
cp "$NATIVE/libmain.so" "$STAGE/lib/arm64-v8a/"

UNALIGNED="$BUILD/vidya-native-unaligned.apk"
ALIGNED="$BUILD/vidya-native-aligned.apk"
APK="$BUILD/vidya-native.apk"
rm -f "$UNALIGNED" "$ALIGNED" "$APK"
"$TOOLS/aapt2" link \
  -o "$UNALIGNED" \
  -I "$ANDROID_HOME/platforms/android-36/android.jar" \
  --manifest "$RAYLIB/android/AndroidManifest.xml" \
  --min-sdk-version 28 \
  --target-sdk-version 36 \
  --version-code 1 \
  --version-name 0.1.0
(cd "$STAGE" && zip -q -0 "$UNALIGNED" lib/arm64-v8a/libmain.so)
"$TOOLS/zipalign" -f 4 "$UNALIGNED" "$ALIGNED"

KEYSTORE="$HOME/.android/debug.keystore"
if [[ ! -f "$KEYSTORE" ]]; then
  mkdir -p "$(dirname "$KEYSTORE")"
  keytool -genkeypair -v \
    -keystore "$KEYSTORE" -storepass android -keypass android \
    -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 \
    -dname "CN=Android Debug,O=Android,C=US"
fi
"$TOOLS/apksigner" sign \
  --ks "$KEYSTORE" --ks-key-alias androiddebugkey \
  --ks-pass pass:android --key-pass pass:android \
  --out "$APK" "$ALIGNED"
"$TOOLS/apksigner" verify --verbose "$APK" >/dev/null

case "${1:-build}" in
  build)
    printf '%s\n' "$APK"
    ;;
  install)
    "$ADB" install -r "$APK"
    ;;
  run)
    "$ADB" install -r "$APK"
    "$ADB" shell am force-stop "$PACKAGE"
    "$ADB" shell am start -n "$ACTIVITY"
    ;;
  log)
    "$ADB" logcat -s raylib
    ;;
  *)
    echo "usage: $0 [build|install|run|log]" >&2
    exit 2
    ;;
esac
