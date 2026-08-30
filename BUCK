
# The egui theme layer. Mirrors the root Cargo.toml package.
rust_library(
    name = "vidya",
    # include_bytes! reads these at compile time, so they belong in srcs
    # rather than resources.
    srcs = glob(["src/**/*.rs"]) + [
        "assets/vidya-symbols.ttf",
        "assets/emoji/twemoji-72x72.zip",
    ],
    crate_root = "src/lib.rs",
    edition = "2021",
    visibility = ["PUBLIC"],
    deps = [
        "//third-party/rust:egui",
        "//third-party/rust:png",
        "//third-party/rust:zip",
    ] + select({
        "DEFAULT": [],
        # `sync_system_chrome_from_android` reads insets off the
        # NativeActivity handle. Mirrors the target-gated dep in Cargo.toml.
        "prelude//os:android": ["//third-party/rust:winit"],
    }),
)

# The C ABI Jolt loads: libvidya.so, matching raylib/'s output name so the
# backend is selected by library search path alone.
rust_library(
    name = "vidya-ffi",
    srcs = glob(["ffi/src/**/*.rs"]),
    crate = "vidya",
    crate_root = "ffi/src/lib.rs",
    edition = "2021",
    preferred_linkage = "shared",
    # Without this buck names the library after the target — lib_vidya-ffi.so —
    # and that name, not the filename, is what the linker writes into a
    # dependent's DT_NEEDED. Every consumer looks for `libvidya`.
    soname = "libvidya.so",
    visibility = ["PUBLIC"],
    # Cargo renames the dependency so the cdylib itself can keep the `vidya`
    # crate name; mirror that here.
    named_deps = {"vidya_core": ":vidya"},
    deps = [
        "//third-party/rust:egui",
        "//third-party/rust:egui-winit",
        "//third-party/rust:egui_glow",
        "//third-party/rust:glow",
        "//third-party/rust:glutin",
        "//third-party/rust:glutin-winit",
        "//third-party/rust:winit",
    ],
)

# Jolt (and the C ABI's other consumers) dlopen `libvidya.so` by name, so give
# the shared library that exact filename in a directory of its own.
genrule(
    name = "libvidya",
    out = "libvidya.so",
    cmd = "cp $(location :vidya-ffi[shared]) $OUT",
    visibility = ["PUBLIC"],
)

# The same C ABI, cross-compiled for a 64-bit Android device. The sources and
# deps are identical — only the target configuration moves, which is what
# `configured_alias` is for.
configured_alias(
    name = "vidya-ffi-android",
    actual = ":vidya-ffi",
    platform = "//platforms:android-arm64",
    visibility = ["PUBLIC"],
)

genrule(
    name = "libvidya-android",
    out = "libvidya.so",
    cmd = "cp $(location :vidya-ffi-android[shared]) $OUT",
    visibility = ["PUBLIC"],
)
