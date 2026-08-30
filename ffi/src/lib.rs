//! Vidya's C ABI, implemented on the Rust/egui semantic layer.
//!
//! This is a third backend behind the header in `raylib/include/vidya.h`,
//! alongside the direct-raylib and cimgui ones. It exports the same symbols
//! from a `cdylib` named `libvidya`, so Jolt — or any other FFI consumer —
//! switches backends by shared-library search path alone, with no binding
//! changes.
//!
//! Rules inherited from the ABI:
//!
//! * one UI context per process;
//! * every call stays on the thread that called `vidya_open` (enforced here:
//!   the context lives in thread-local storage, so calls from other threads are
//!   inert rather than unsound);
//! * only C integers, floats, pointers, and UTF-8 byte strings cross the
//!   boundary, and nothing on this side retains caller memory past the call.
//!
//! Panics are caught at the boundary: unwinding into a C or Chez caller would
//! be undefined behaviour.

mod app;
mod ui;

use std::cell::RefCell;
use std::ffi::{c_char, c_float, c_int, CStr};
use std::panic::AssertUnwindSafe;

use app::App;
use egui::Ui;
use vidya_core::{Mode, Theme};

thread_local! {
    /// The process's UI context, owned by the thread that opened the window.
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn guard<R>(fallback: R, f: impl FnOnce() -> R) -> R {
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("vidya: panic caught at the FFI boundary");
            fallback
        }
    }
}

fn with_app<R: Copy>(fallback: R, f: impl FnOnce(&mut App) -> R) -> R {
    guard(fallback, || {
        APP.with_borrow_mut(|slot| match slot.as_mut() {
            Some(app) => f(app),
            None => fallback,
        })
    })
}

/// Run `f` against the innermost open UI node. Inert outside a frame.
fn with_ui<R: Copy>(fallback: R, f: impl FnOnce(&mut Ui, &Theme) -> R) -> R {
    with_app(fallback, |app| match app.ui() {
        Some((ui, theme)) => f(ui, theme),
        None => fallback,
    })
}

/// # Safety
/// `ptr` is null or a NUL-terminated string valid for the duration of the call.
unsafe fn borrowed_str(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// Copy `value` into a caller buffer, truncated at a char boundary and always
/// NUL-terminated.
///
/// # Safety
/// `buf` is null or writable for `capacity` bytes.
unsafe fn write_buffer(buf: *mut c_char, capacity: usize, value: &str) {
    if buf.is_null() || capacity == 0 {
        return;
    }
    let mut len = value.len().min(capacity - 1);
    while len > 0 && !value.is_char_boundary(len) {
        len -= 1;
    }
    std::ptr::copy_nonoverlapping(value.as_ptr().cast::<c_char>(), buf, len);
    *buf.add(len) = 0;
}

// ── Window and frame lifecycle ──────────────────────────────────────────────

/// # Safety
/// `title` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_open(width: c_int, height: c_int, title: *const c_char) -> c_int {
    let title = borrowed_str(title);
    guard(0, || {
        APP.with_borrow_mut(|slot| {
            if slot.is_some() {
                eprintln!("vidya: a window is already open");
                return 0;
            }
            match App::open(width, height, &title) {
                Ok(app) => {
                    *slot = Some(app);
                    1
                }
                Err(e) => {
                    eprintln!("vidya: could not open a window: {e}");
                    0
                }
            }
        })
    })
}

#[no_mangle]
pub extern "C" fn vidya_close() {
    guard((), || APP.with_borrow_mut(|slot| drop(slot.take())));
}

#[no_mangle]
pub extern "C" fn vidya_should_close() -> c_int {
    // No window is a closed window, so a caller's loop still terminates.
    with_app(1, |app| app.should_close() as c_int)
}

#[no_mangle]
pub extern "C" fn vidya_set_target_fps(fps: c_int) {
    with_app((), |app| app.set_target_fps(fps));
}

#[no_mangle]
pub extern "C" fn vidya_set_mode(mode: c_int) {
    let mode = if mode == 1 { Mode::Light } else { Mode::Dark };
    with_app((), |app| app.set_mode(mode));
}

#[no_mangle]
pub extern "C" fn vidya_get_mode() -> c_int {
    with_app(0, |app| match app.theme().mode {
        Mode::Dark => 0,
        Mode::Light => 1,
    })
}

/// `atlas_size` is accepted for ABI compatibility and ignored: egui rasterizes
/// each requested size on demand instead of from one fixed atlas.
///
/// # Safety
/// `path` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_load_font(path: *const c_char, _atlas_size: c_int) -> c_int {
    let path = borrowed_str(path);
    with_app(0, |app| app.load_font(&path) as c_int)
}

#[no_mangle]
pub extern "C" fn vidya_begin_frame() {
    with_app((), |app| app.begin_frame());
}

#[no_mangle]
pub extern "C" fn vidya_end_frame() {
    with_app((), |app| app.end_frame());
}

// ── Containers ──────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn vidya_page_begin(max_width: c_float) {
    with_app((), |app| {
        let theme = app.theme().clone();
        app.stack.push_page(&theme, max_width);
    });
}

#[no_mangle]
pub extern "C" fn vidya_page_end() {
    with_app((), |app| app.stack.pop());
}

#[no_mangle]
pub extern "C" fn vidya_card_begin() {
    with_app((), |app| {
        let theme = app.theme().clone();
        app.stack.push_card(&theme);
    });
}

#[no_mangle]
pub extern "C" fn vidya_card_end() {
    with_app((), |app| app.stack.pop());
}

#[no_mangle]
pub extern "C" fn vidya_gap(pixels: c_float) {
    with_ui((), |ui, _| ui::gap(ui, pixels));
}

#[no_mangle]
pub extern "C" fn vidya_separator() {
    with_ui((), |ui, _| ui::separator(ui));
}

// ── Text roles ──────────────────────────────────────────────────────────────

macro_rules! text_role {
    ($name:ident, $call:path) => {
        /// # Safety
        /// `text` is null or a NUL-terminated UTF-8 string.
        #[no_mangle]
        pub unsafe extern "C" fn $name(text: *const c_char) {
            let text = borrowed_str(text);
            with_ui((), |ui, theme| $call(ui, theme, &text));
        }
    };
}

text_role!(vidya_title, vidya_core::title);
text_role!(vidya_title_2, vidya_core::title_2);
text_role!(vidya_body, vidya_core::body);
text_role!(vidya_dim_label, vidya_core::dim_label);

// ── Controls ────────────────────────────────────────────────────────────────

/// # Safety
/// `label` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_button(label: *const c_char, kind: c_int) -> c_int {
    let label = borrowed_str(label);
    with_ui(0, |ui, theme| ui::button(ui, theme, &label, kind) as c_int)
}

/// Returns 1 when the value changed this frame, writing it back through
/// `checked`.
///
/// # Safety
/// `label` is null or a NUL-terminated UTF-8 string; `checked` is null or a
/// writable `int`.
#[no_mangle]
pub unsafe extern "C" fn vidya_checkbox(label: *const c_char, checked: *mut c_int) -> c_int {
    if checked.is_null() {
        return 0;
    }
    let label = borrowed_str(label);
    let current = *checked != 0;
    let (value, changed) = with_ui((current, false), |ui, theme| {
        ui::checkbox(ui, theme, current, &label)
    });
    *checked = value as c_int;
    changed as c_int
}

/// FFI-friendly variant: returns the value after handling input.
///
/// # Safety
/// `label` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_checkbox_value(label: *const c_char, checked: c_int) -> c_int {
    let label = borrowed_str(label);
    let current = checked != 0;
    with_ui(current, |ui, theme| ui::checkbox(ui, theme, current, &label).0) as c_int
}

/// # Safety
/// `label` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_status(label: *const c_char, live: c_int) {
    let label = borrowed_str(label);
    with_ui((), |ui, theme| ui::status(ui, theme, &label, live != 0));
}

/// Edit `text` in place. Returns 1 when the buffer changed this frame.
///
/// # Safety
/// `text` is null or a NUL-terminated buffer writable for `capacity` bytes;
/// `placeholder` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_text_field(
    text: *mut c_char,
    capacity: usize,
    placeholder: *const c_char,
) -> c_int {
    if text.is_null() || capacity == 0 {
        return 0;
    }
    let placeholder = borrowed_str(placeholder);
    let mut value = borrowed_str(text.cast_const());

    let changed = with_ui(false, |ui, theme| {
        ui::text_field(ui, theme, &mut value, &placeholder)
    });
    if changed {
        write_buffer(text, capacity, &value);
    }
    changed as c_int
}
