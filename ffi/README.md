# Vidya for Rust (C ABI)

This directory implements the C ABI in [`../raylib/include/vidya.h`](../raylib/include/vidya.h)
on top of the **Rust/egui** semantic layer in [`../src`](../src) — the same
palette, spacing, type scale, and widgets the desktop and Android demos use.

It is a third backend behind one header, alongside the direct-raylib and cimgui
ones. It builds a `cdylib` named `libvidya`, so consumers switch backends by
shared-library search path alone:

```sh
cargo build --manifest-path ffi/Cargo.toml --release

cd jolt
LD_LIBRARY_PATH=../ffi/target/release jolt -M:showcase   # widget showcase
LD_LIBRARY_PATH=../ffi/target/release jolt -M:app        # stateful Control Center
```

No Jolt binding changes: `jolt/src/vidya/core.jolt` is unmodified, and
`deps.edn` already declares `libvidya.so`.

## How the pull-style ABI maps onto egui

The ABI is caller-driven — the caller owns the loop and calls
`vidya_begin_frame` / `vidya_end_frame`. `eframe` inverts that, so this backend
skips it and drives `winit` with `pump_app_events`, painting through
`egui_glow`.

The ABI is also push/pop (`vidya_card_begin` … `vidya_card_end`) while the Rust
API is closure-based (`vidya::card(ui, theme, |ui| …)`). `src/ui.rs` bridges the
two with a stack of live `egui::Ui` nodes, closing each into its parent with
`Frame::begin`/`Prepared::end` and `Ui::new_child`, so layout matches the
closure form.

Panics are caught at the boundary — unwinding into a C or Chez caller is
undefined behaviour — and the context lives in thread-local storage, so the
ABI's "stay on the window thread" rule is enforced rather than assumed.

## X11 is preferred on Linux

The event loop asks for X11 first (XWayland counts), falling back to Wayland.

Native Wayland does not survive this ABI's shape. The caller owns the loop, so
winit must be driven with `pump_app_events`, and a Wayland surface driven that
way stops receiving frame callbacks after its first commit: the window never
gets a second frame, and `swap_buffers` blocks inside EGL holding the thread
that owes the compositor its replies — which stalls the whole session, not just
this window. `eframe` is unaffected only because `run_app` owns its loop.

Consequence: fractional scaling and per-monitor DPI follow XWayland's rules
here. Apps that need native Wayland should use the Rust crate directly with
`eframe`, which has no such constraint.

## Coverage

Everything in `vidya.h` is implemented. Two notes:

* `vidya_load_font`'s `atlas_size` is accepted and ignored — egui rasterizes
  each size on demand rather than from one fixed atlas. The symbol-font
  fallback stays installed, so `→ ● ▾ █` keep rendering.
* The page is not scrollable: `egui::ScrollArea` has no public push/pop form to
  hold open across FFI calls. Content taller than the window is clipped.

The grid DSL (`central_page` / `page_body` / `grid_cols`) is not exposed — the
ABI's flat vertical cursor has no equivalent. Adding it means new ABI calls.

## Verifying a render

`VIDYA_CAPTURE=<path>` writes the third painted frame as a binary PPM. A
rendering backend is otherwise unverifiable in CI, or on a desktop whose
compositor refuses screenshots.

```sh
VIDYA_CAPTURE=/tmp/frame.ppm LD_LIBRARY_PATH=ffi/target/release jolt -M:showcase
```
