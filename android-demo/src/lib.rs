//! Shared library: desktop binary + Android NativeActivity (`android_main`).

mod app;
mod gleam_app;
mod gleam_bridge;

pub use app::run_desktop;
pub use gleam_app::run_gleam_app;
pub use gleam_bridge::{
    install_gleam_gui, install_gleam_shell, GleamGuiHooks, GleamShellHooks,
};

#[cfg(target_os = "android")]
pub use app::run_android;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    log::info!("vidya-demo android_main start");
    // If run_native returns, the activity finishes (looks like a one-frame flash).
    match run_android(android_app) {
        Ok(()) => log::info!("vidya-demo run_android returned Ok (window closed)"),
        Err(e) => log::error!("vidya-demo run_android error: {e}"),
    }
    log::info!("vidya-demo android_main end");
}
