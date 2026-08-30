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

#[cfg(target_os = "android")]
mod android;
mod app;
mod tree;
mod ui;

use std::cell::RefCell;
use std::ffi::{c_char, c_float, c_int, CStr, CString};
use std::panic::AssertUnwindSafe;

use app::App;
use egui::Ui;
use tree::{Tree, Value};
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
        ui::text_field(ui, theme, &mut value, &placeholder).changed()
    });
    if changed {
        write_buffer(text, capacity, &value);
    }
    changed as c_int
}

// ── Retained node tree ──────────────────────────────────────────────────────
//
// The second half of this ABI, for reactive callers. See `tree.rs` for why it
// exists and `include/vidya_tree.h` for the contract. Everything below is inert
// until the caller builds a tree; a program using only the push/pop calls above
// never allocates one.

thread_local! {
    /// The node tree, on the same thread as the window by the same rule as
    /// `APP`. Created on first use — a push/pop caller never pays for it.
    static TREE: RefCell<Tree> = RefCell::new(Tree::default());

    /// Backing store for the `const char *` returns below. Rust owns every
    /// string that crosses this boundary, so it has to outlive the call that
    /// returns it without leaking: one slot, overwritten by the next call.
    static SCRATCH: RefCell<CString> = RefCell::new(CString::default());
}

fn with_tree<R: Copy>(fallback: R, f: impl FnOnce(&mut Tree) -> R) -> R {
    guard(fallback, || TREE.with_borrow_mut(f))
}

/// Copy `value` into the scratch slot and return a pointer C can read until the
/// next string-returning call. Interior NULs truncate rather than fail.
fn scratch(value: &str) -> *const c_char {
    let owned = CString::new(value).unwrap_or_else(|e| {
        let mut bytes = e.into_vec();
        bytes.truncate(bytes.iter().position(|&b| b == 0).unwrap_or(0));
        CString::new(bytes).expect("truncated at the first NUL")
    });
    SCRATCH.with_borrow_mut(|slot| {
        *slot = owned;
        slot.as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn vidya_tree_root() -> c_int {
    with_tree(0, |tree| tree.root() as c_int)
}

/// # Safety
/// `tag` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_node_new(tag: *const c_char) -> c_int {
    let tag = borrowed_str(tag);
    with_tree(0, |tree| tree.new_node(&tag) as c_int)
}

#[no_mangle]
pub extern "C" fn vidya_node_free(node: c_int) {
    with_tree((), |tree| tree.free_node(node.max(0) as u32));
}

#[no_mangle]
pub extern "C" fn vidya_node_exists(node: c_int) -> c_int {
    with_tree(0, |tree| tree.exists(node.max(0) as u32) as c_int)
}

/// # Safety
/// `key` and `value` are null or NUL-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn vidya_node_set_str(node: c_int, key: *const c_char, value: *const c_char) {
    let (key, value) = (borrowed_str(key), borrowed_str(value));
    with_tree((), |tree| {
        tree.set(node.max(0) as u32, &key, Value::Str(value))
    });
}

/// # Safety
/// `key` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_node_set_num(node: c_int, key: *const c_char, value: f64) {
    let key = borrowed_str(key);
    with_tree((), |tree| {
        tree.set(node.max(0) as u32, &key, Value::Num(value))
    });
}

/// # Safety
/// `key` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_node_set_bool(node: c_int, key: *const c_char, value: c_int) {
    let key = borrowed_str(key);
    with_tree((), |tree| {
        tree.set(node.max(0) as u32, &key, Value::Bool(value != 0))
    });
}

/// Drop every prop, so a re-render starts from a clean slate rather than
/// inheriting props the new hiccup no longer sets.
#[no_mangle]
pub extern "C" fn vidya_node_clear_props(node: c_int) {
    with_tree((), |tree| tree.clear_props(node.max(0) as u32));
}

/// The empty string for a prop that is unset or is not a string.
///
/// # Safety
/// `key` is null or a NUL-terminated UTF-8 string. The returned pointer is
/// valid until the next string-returning call on this thread.
#[no_mangle]
pub unsafe extern "C" fn vidya_node_get_str(node: c_int, key: *const c_char) -> *const c_char {
    let key = borrowed_str(key);
    let value = TREE.with_borrow(|tree| match tree.get(node.max(0) as u32, &key) {
        Some(Value::Str(s)) => s.clone(),
        _ => String::new(),
    });
    scratch(&value)
}

/// # Safety
/// `key` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_node_get_num(node: c_int, key: *const c_char) -> f64 {
    let key = borrowed_str(key);
    with_tree(0.0, |tree| match tree.get(node.max(0) as u32, &key) {
        Some(Value::Num(n)) => *n,
        Some(Value::Bool(true)) => 1.0,
        _ => 0.0,
    })
}

/// # Safety
/// `key` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_node_get_bool(node: c_int, key: *const c_char) -> c_int {
    let key = borrowed_str(key);
    with_tree(0, |tree| {
        (match tree.get(node.max(0) as u32, &key) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Num(n)) => *n != 0.0,
            _ => false,
        }) as c_int
    })
}

/// The canonical tag name a node was created with; `hbox` and `vbox` both
/// answer `box`. The empty string for a node that no longer exists.
///
/// # Safety
/// The returned pointer is valid until the next string-returning call on this
/// thread.
#[no_mangle]
pub extern "C" fn vidya_node_tag(node: c_int) -> *const c_char {
    let tag = TREE.with_borrow(|tree| tree.tag_name(node.max(0) as u32));
    scratch(tag)
}

#[no_mangle]
pub extern "C" fn vidya_node_child_count(node: c_int) -> c_int {
    with_tree(0, |tree| tree.child_count(node.max(0) as u32) as c_int)
}

/// The `index`th child, or 0 when there is none.
#[no_mangle]
pub extern "C" fn vidya_node_child_at(node: c_int, index: c_int) -> c_int {
    with_tree(0, |tree| {
        tree.child_at(node.max(0) as u32, index.max(0) as usize) as c_int
    })
}

#[no_mangle]
pub extern "C" fn vidya_node_append(parent: c_int, child: c_int) -> c_int {
    with_tree(0, |tree| {
        tree.append(parent.max(0) as u32, child.max(0) as u32) as c_int
    })
}

/// Unparent `child` **and free it**, with everything under it.
///
/// glimmer says nothing further about a widget it has removed, so this is where
/// a subtree's storage goes back.
#[no_mangle]
pub extern "C" fn vidya_node_remove(parent: c_int, child: c_int) {
    with_tree((), |tree| {
        tree.remove(parent.max(0) as u32, child.max(0) as u32)
    });
}

/// Move `child` to sit immediately after `sibling`; `sibling` 0 means first.
#[no_mangle]
pub extern "C" fn vidya_node_insert_after(parent: c_int, child: c_int, sibling: c_int) -> c_int {
    with_tree(0, |tree| {
        tree.insert_after(
            parent.max(0) as u32,
            child.max(0) as u32,
            sibling.max(0) as u32,
        ) as c_int
    })
}

/// Put `new_child` where `old_child` was, and free `old_child`.
#[no_mangle]
pub extern "C" fn vidya_node_replace(parent: c_int, old_child: c_int, new_child: c_int) -> c_int {
    with_tree(0, |tree| {
        tree.replace(
            parent.max(0) as u32,
            old_child.max(0) as u32,
            new_child.max(0) as u32,
        ) as c_int
    })
}

/// Paint the whole tree as one frame: a `vidya_begin_frame`, the walk, and a
/// `vidya_end_frame`. Inert with no window open.
#[no_mangle]
pub extern "C" fn vidya_tree_frame() {
    with_app((), |app| {
        app.begin_frame();
        TREE.with_borrow_mut(|tree| {
            if let Some((ui, theme)) = app.ui() {
                tree.paint(ui, theme);
            }
        });
        app.end_frame();
    });
}

/// Dequeue one event, returning 1 while there was one. Its fields are read with
/// the accessors below, which describe the most recently dequeued event.
#[no_mangle]
pub extern "C" fn vidya_tree_poll_event() -> c_int {
    with_tree(0, |tree| tree.poll() as c_int)
}

#[no_mangle]
pub extern "C" fn vidya_tree_event_node() -> c_int {
    with_tree(0, |tree| tree.current().map_or(0, |e| e.node) as c_int)
}

/// The event's name — `click`, `change`, `toggled`, `activate` — or the empty
/// string when nothing has been dequeued.
///
/// # Safety
/// The returned pointer is valid until the next string-returning call on this
/// thread.
#[no_mangle]
pub extern "C" fn vidya_tree_event_name() -> *const c_char {
    let name = TREE.with_borrow(|tree| tree.current().map_or("", |e| e.name).to_owned());
    scratch(&name)
}

/// # Safety
/// The returned pointer is valid until the next string-returning call on this
/// thread.
#[no_mangle]
pub extern "C" fn vidya_tree_event_text() -> *const c_char {
    let text = TREE.with_borrow(|tree| tree.current().map_or(String::new(), |e| e.text.clone()));
    scratch(&text)
}

#[no_mangle]
pub extern "C" fn vidya_tree_event_num() -> f64 {
    with_tree(0.0, |tree| tree.current().map_or(0.0, |e| e.num))
}

// ── Clipboard ───────────────────────────────────────────────────────────────

/// Write the picture on the system clipboard to `path` as a PNG, answering 1
/// when there was one and it was written.
///
/// egui carries clipboard *text* into the frame as an event and nothing else,
/// so a pasted image has to be asked for rather than waited for: a caller
/// binds this to whatever gesture means paste for it, and reads the file.
/// PNG because that is what the `:image` node decodes.
///
/// Unlike the rest of this ABI it needs no window and no particular thread —
/// it talks to the platform clipboard, not to egui.
///
/// # Safety
/// `path` is null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn vidya_clipboard_image_png(path: *const c_char) -> c_int {
    let path = borrowed_str(path);
    guard(0, || {
        if path.is_empty() {
            return 0;
        }
        clipboard_image_png(&path) as c_int
    })
}

#[cfg(not(target_os = "android"))]
fn clipboard_image_png(path: &str) -> bool {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return false;
    };
    // An empty clipboard, text on it, or a format the platform will not hand
    // over as pixels: all of them are "no picture to paste" to the caller.
    let Ok(image) = clipboard.get_image() else {
        return false;
    };
    let Ok(file) = std::fs::File::create(path) else {
        return false;
    };
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        image.width as u32,
        image.height as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let written = encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&image.bytes))
        .is_ok();
    // A half-written file is worse than none: the caller would upload it.
    if !written {
        let _ = std::fs::remove_file(path);
    }
    written
}

/// Android has no clipboard of images to read, and arboard no backend for it.
#[cfg(target_os = "android")]
fn clipboard_image_png(_path: &str) -> bool {
    false
}
