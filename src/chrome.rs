//! System chrome (status bar / nav bar) safe areas.
//!
//! Android NativeActivity draws **edge-to-edge**: the window fills under the
//! system status bar (clock, battery, …) and the gesture/nav bar. Without an
//! explicit reserve, app chrome and labels sit under those system widgets.
//!
//! Call [`reserve_system_chrome`] once at the start of every frame (before any
//! other `TopBottomPanel` / `SidePanel` / `CentralPanel`), or use
//! [`top_header`] which does that for you.

use egui::{Context, Frame, Margin, Sense, Ui};

use crate::Theme;

/// Insets for system status / navigation chrome on edge-to-edge surfaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemChrome {
    /// Space under the status bar (clock, indicators).
    pub top: f32,
    /// Space above the system gesture / 3-button nav bar.
    pub bottom: f32,
}

impl SystemChrome {
    pub const ZERO: Self = Self {
        top: 0.0,
        bottom: 0.0,
    };

    pub fn is_zero(self) -> bool {
        self.top <= 0.0 && self.bottom <= 0.0
    }
}

/// Platform defaults for edge-to-edge drawing.
///
/// Values match common gesture-nav phones (≈24–36 dp status, ≈48 dp nav).
/// Override later if/when real `WindowInsets` are plumbed through egui.
pub fn system_chrome(_ctx: &Context) -> SystemChrome {
    #[cfg(target_os = "android")]
    {
        SystemChrome {
            top: 36.0,
            bottom: 48.0,
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        SystemChrome::ZERO
    }
}

/// Reserve top/bottom strips so **no** subsequent panel or central content can
/// paint under the system status or navigation bars.
///
/// Call this **once per frame**, before other panels. Safe to call when insets
/// are zero (no-op on desktop).
///
/// The reserved bands use the theme headerbar fill so they blend with a top
/// header sitting just below the status bar.
pub fn reserve_system_chrome(ctx: &Context, theme: &Theme) {
    let chrome = system_chrome(ctx);
    if chrome.is_zero() {
        return;
    }

    let fill = theme.palette.headerbar_bg;
    let band = Frame::new().fill(fill).inner_margin(Margin::ZERO);

    if chrome.top > 0.0 {
        egui::TopBottomPanel::top("vidya_system_chrome_top")
            .exact_height(chrome.top)
            .frame(band)
            .show_separator_line(false)
            .show(ctx, |_ui| {});
    }

    if chrome.bottom > 0.0 {
        egui::TopBottomPanel::bottom("vidya_system_chrome_bottom")
            .exact_height(chrome.bottom)
            .frame(band)
            .show_separator_line(false)
            .show(ctx, |ui| {
                // Absorb input so gestures aren't stolen by widgets behind the band.
                ui.allocate_exact_size(ui.available_size(), Sense::hover());
            });
    }
}

/// Top app header with system status bar already reserved.
///
/// Preferred entry for shell chrome: apps cannot place title/status text under
/// the clock / indicators. Also reserves the bottom system nav band.
///
/// ```ignore
/// vidya::top_header(ctx, &theme, |ui| {
///     ui.horizontal(|ui| {
///         vidya::title(ui, &theme, "My App");
///     });
/// });
/// ```
pub fn top_header(ctx: &Context, theme: &Theme, add_contents: impl FnOnce(&mut Ui)) {
    reserve_system_chrome(ctx, theme);
    egui::TopBottomPanel::top("vidya_app_header")
        .frame(theme.header_frame())
        .show_separator_line(false)
        .show(ctx, add_contents);
}
