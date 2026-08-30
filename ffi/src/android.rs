//! The Android entry point, and the `AndroidApp` the frame loop needs.
//!
//! On a desktop the caller owns `main` and this library is just a dependency.
//! Android inverts that: the activity is created by the platform, and whoever
//! owns the activity owns the event loop. winit cannot build one on Android
//! without the `AndroidApp` handle that `android-activity`'s glue receives, and
//! that glue only runs if *this* library is the NativeActivity's library.
//!
//! So on Android the roles swap. `libvidya.so` is the entry point named by the
//! manifest, [`android_main`] stashes the handle [`build_event_loop`] later
//! needs, and then hands control to the application — an embedded Jolt boot
//! image in `libjoltapp.so`, which calls straight back into the C ABI below.
//!
//! The application is reached by `dlopen` rather than by linking. Linking would
//! be a cycle — `libjoltapp.so` already needs this library for every `vidya_*`
//! symbol it registers with Chez — and Android's loader resolves a dlopened
//! library's dependencies without one.

use std::ffi::{c_char, c_int, c_void, CString};
use std::sync::Mutex;

use winit::platform::android::activity::AndroidApp;

/// The library holding the application's `vidya_jolt_main`.
const APP_LIBRARY: &str = "libjoltapp.so";

/// Set once, before any application code runs, and read on the loop thread.
static ANDROID_APP: Mutex<Option<AndroidApp>> = Mutex::new(None);

/// The handle winit needs to build an event loop. `None` off Android's own
/// thread, or before the glue has started.
pub fn android_app() -> Option<AndroidApp> {
    ANDROID_APP.lock().ok()?.clone()
}

extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

const ANDROID_LOG_INFO: c_int = 4;
const ANDROID_LOG_ERROR: c_int = 6;
const RTLD_NOW: c_int = 2;

fn log(priority: c_int, message: &str) {
    let (Ok(tag), Ok(text)) = (CString::new("Vidya"), CString::new(message)) else {
        return;
    };
    // SAFETY: both pointers are NUL-terminated and live across the call.
    unsafe { __android_log_write(priority, tag.as_ptr(), text.as_ptr()) };
}

fn last_dl_error() -> String {
    // SAFETY: dlerror returns a borrowed C string, or null when nothing failed.
    let err = unsafe { dlerror() };
    if err.is_null() {
        return "unknown error".to_owned();
    }
    unsafe { std::ffi::CStr::from_ptr(err) }
        .to_string_lossy()
        .into_owned()
}

/// Entry point for `android-activity`'s NativeActivity glue.
///
/// Returning from here finishes the activity, which looks like a one-frame
/// flash, so a failure to reach the application is logged rather than silent.
#[no_mangle]
pub extern "C" fn android_main(app: AndroidApp) {
    if let Ok(mut slot) = ANDROID_APP.lock() {
        *slot = Some(app);
    }

    log(ANDROID_LOG_INFO, "vidya: loading the Jolt application");

    let Ok(name) = CString::new(APP_LIBRARY) else {
        return;
    };
    // SAFETY: `name` is a NUL-terminated library name; the handle is only used
    // to look up one symbol and is deliberately never closed — the application
    // runs for the lifetime of the process.
    let handle = unsafe { dlopen(name.as_ptr(), RTLD_NOW) };
    if handle.is_null() {
        log(
            ANDROID_LOG_ERROR,
            &format!("vidya: cannot load {APP_LIBRARY}: {}", last_dl_error()),
        );
        return;
    }

    let Ok(symbol) = CString::new("vidya_jolt_main") else {
        return;
    };
    // SAFETY: as above; the result is checked before it is called.
    let entry = unsafe { dlsym(handle, symbol.as_ptr()) };
    if entry.is_null() {
        log(
            ANDROID_LOG_ERROR,
            &format!(
                "vidya: {APP_LIBRARY} has no vidya_jolt_main: {}",
                last_dl_error()
            ),
        );
        return;
    }

    // SAFETY: `vidya_jolt_main` is declared by the glue in android/jolt_main.c
    // as `int vidya_jolt_main(void)`; it boots Chez and does not return until
    // the application exits.
    let entry: extern "C" fn() -> c_int = unsafe { std::mem::transmute(entry) };
    let status = entry();
    log(
        ANDROID_LOG_INFO,
        &format!("vidya: Jolt application exited with status {status}"),
    );
}
