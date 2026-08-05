//! System chrome (status bar / nav bar) safe areas.
//!
//! Android NativeActivity draws **edge-to-edge**: the window fills under the
//! system status bar (clock, battery, …) and the gesture/nav bar. Without an
//! explicit reserve, app chrome and labels sit under those system widgets.
//!
//! Call [`reserve_system_chrome`] once at the start of every frame (before any
//! other `TopBottomPanel` / `SidePanel` / `CentralPanel`), or use
//! [`top_header`] which does that for you.
//!
//! Prefer injecting measured insets via [`set_system_chrome`] or
//! [`sync_system_chrome_from_android`] (from `AndroidApp::content_rect` or
//! WindowInsets) so reserves match the device — hardcoded fallbacks are
//! intentionally tight for modern gesture-nav phones.

use egui::{Align2, Context, Frame, Id, Margin, Sense, Ui, WidgetText, Window};

use crate::Theme;

/// Temp-data id for app-supplied measured insets (see [`set_system_chrome`]).
const SYSTEM_CHROME_ID: &str = "vidya.system_chrome";
/// Last measured nav-band height when the keyboard was hidden.
const NAV_HINT_ID: &str = "vidya.nav_bottom_hint";

/// Insets for system status / navigation chrome on edge-to-edge surfaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemChrome {
    /// Space under the status bar (clock, indicators).
    pub top: f32,
    /// Space above the system gesture / 3-button nav bar.
    pub nav_bottom: f32,
    /// Extra reserve when the soft keyboard is visible (sits above [`nav_bottom`]).
    pub ime_bottom: f32,
}

impl SystemChrome {
    pub const ZERO: Self = Self {
        top: 0.0,
        nav_bottom: 0.0,
        ime_bottom: 0.0,
    };

    /// Combined bottom inset (nav + IME).
    #[inline]
    pub fn bottom(self) -> f32 {
        self.nav_bottom + self.ime_bottom
    }

    pub fn is_zero(self) -> bool {
        self.top <= 0.0 && self.bottom() <= 0.0
    }

    /// Build from status + nav insets (no IME reserve).
    pub fn from_insets(top: f32, nav_bottom: f32) -> Self {
        Self {
            top: top.max(0.0),
            nav_bottom: nav_bottom.max(0.0),
            ime_bottom: 0.0,
        }
    }
}

/// Inject measured system insets for this frame (egui points).
///
/// Call once per frame **before** [`reserve_system_chrome`]. When set,
/// [`system_chrome`] uses these values instead of the platform fallbacks
/// (IME expansion still applies on top of `nav_bottom`).
pub fn set_system_chrome(ctx: &Context, chrome: SystemChrome) {
    ctx.data_mut(|d| d.insert_temp(Id::new(SYSTEM_CHROME_ID), chrome));
}

/// Read measured insets from [`AndroidApp::content_rect`] and call
/// [`set_system_chrome`].
///
/// Call once per frame on Android **before** [`reserve_system_chrome`]. When the
/// soft keyboard is open, `content_rect` shrinks and the derived bottom inset
/// includes the IME height so layout tracks the real keyboard band.
#[cfg(target_os = "android")]
pub fn sync_system_chrome_from_android(
    ctx: &Context,
    app: &winit::platform::android::activity::AndroidApp,
) {
    const NAV_FALLBACK: f32 = 20.0;

    let rect = app.content_rect();
    let ppp = ctx.pixels_per_point().max(0.01);
    let screen = ctx.screen_rect();

    let top = (rect.top as f32 / ppp).max(0.0);
    let content_bottom_pt = rect.bottom as f32 / ppp;
    let total_bottom = (screen.height() - content_bottom_pt).max(0.0);

    if ctx.wants_keyboard_input() {
        let nav = ctx
            .data(|d| d.get_temp::<f32>(Id::new(NAV_HINT_ID)))
            .unwrap_or(NAV_FALLBACK)
            .clamp(0.0, total_bottom);
        set_system_chrome(
            ctx,
            SystemChrome {
                top,
                nav_bottom: nav,
                ime_bottom: (total_bottom - nav).max(0.0),
            },
        );
    } else {
        ctx.data_mut(|d| d.insert_temp(Id::new(NAV_HINT_ID), total_bottom));
        set_system_chrome(ctx, SystemChrome::from_insets(top, total_bottom));
    }
}

/// Platform defaults for edge-to-edge drawing.
///
/// Fallback values are **tight** for modern gesture-nav phones (≈24–28 dp
/// status, ≈16–24 dp gesture handle). Prefer [`set_system_chrome`] or
/// [`sync_system_chrome_from_android`] with measured `content_rect` /
/// WindowInsets when available.
///
/// When a text field holds focus (`Context::wants_keyboard_input`), the bottom
/// inset grows so bottom bars / compose fields sit **above** the soft keyboard
/// (NativeActivity rarely resizes the GL surface for IME).
pub fn system_chrome(ctx: &Context) -> SystemChrome {
    #[cfg(target_os = "android")]
    {
        let measured = ctx.data(|d| d.get_temp::<SystemChrome>(Id::new(SYSTEM_CHROME_ID)));
        const TOP_FALLBACK: f32 = 36.0;
        const NAV_FALLBACK: f32 = 20.0;
        let top = match measured {
            Some(c) if c.top >= 8.0 => c.top,
            Some(c) => c.top.max(TOP_FALLBACK),
            None => TOP_FALLBACK,
        };
        let measured_nav = measured.map(|c| c.nav_bottom.max(0.0));
        let mut nav_bottom = measured_nav.unwrap_or(NAV_FALLBACK);
        let mut ime_bottom = 0.0;

        if ctx.wants_keyboard_input() {
            let h = ctx.screen_rect().height();
            let ime_fallback = (h * 0.40).clamp(240.0, h * 0.52);
            if let Some(m) = measured.filter(|c| c.ime_bottom > 0.0) {
                nav_bottom = m.nav_bottom;
                ime_bottom = m.ime_bottom.max(ime_fallback - nav_bottom.max(0.0));
            } else {
                let total_bottom = measured
                    .map(|c| c.bottom().max(ime_fallback))
                    .unwrap_or(ime_fallback);
                nav_bottom = measured_nav.unwrap_or(NAV_FALLBACK).min(total_bottom);
                ime_bottom = (total_bottom - nav_bottom).max(0.0);
            }
            ctx.request_repaint();
        } else {
            nav_bottom = measured_nav.unwrap_or(NAV_FALLBACK);
        }

        SystemChrome {
            top,
            nav_bottom,
            ime_bottom,
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = ctx;
        SystemChrome::ZERO
    }
}

/// Reserve top/bottom strips so **no** subsequent panel or central content can
/// paint under the system status or navigation bars.
///
/// Call this **once per frame**, before other panels. Safe to call when insets
/// are zero (no-op on desktop).
///
/// When the soft keyboard is open, the IME reserve does **not** absorb pointer
/// events so swipe typing on the system keyboard is not blocked. Only the nav
/// strip keeps a hover sink so widgets behind the gesture bar cannot steal taps.
pub fn reserve_system_chrome(ctx: &Context, theme: &Theme) {
    let chrome = system_chrome(ctx);
    if chrome.is_zero() {
        return;
    }

    let top_band = Frame::new()
        .fill(theme.palette.headerbar_bg)
        .inner_margin(Margin::ZERO);
    let nav_band = Frame::new()
        .fill(theme.palette.window_bg)
        .inner_margin(Margin::ZERO);
    // Transparent — only reserves layout; must not paint over the IME.
    let ime_band = Frame::NONE;

    if chrome.top > 0.0 {
        egui::TopBottomPanel::top("vidya_system_chrome_top")
            .exact_height(chrome.top)
            .frame(top_band)
            .show_separator_line(false)
            .show(ctx, |_ui| {});
    }

    // Bottom panels stack upward: declare nav first (screen edge), then IME.
    if chrome.nav_bottom > 0.0 {
        egui::TopBottomPanel::bottom("vidya_system_chrome_nav")
            .exact_height(chrome.nav_bottom)
            .frame(nav_band)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.allocate_exact_size(ui.available_size(), Sense::hover());
            });
    }

    if chrome.ime_bottom > 0.0 {
        egui::TopBottomPanel::bottom("vidya_system_chrome_ime")
            .exact_height(chrome.ime_bottom)
            .frame(ime_band)
            .show_separator_line(false)
            .show(ctx, |ui| {
                // Pass touches through to the system keyboard (swipe / glide typing).
                ui.allocate_exact_size(ui.available_size(), Sense::empty());
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

/// Centered modal-style window with themed card chrome.
///
/// Defaults: non-collapsible, **resizable**, centered, [`Theme::card_frame`].
/// Chain `.default_size` / `.min_width` / `.resizable(false)` as needed, then
/// `.show`.
///
/// ```ignore
/// vidya::dialog("Rename", &theme)
///     .default_width(360.0)
///     .min_width(280.0)
///     .show(ctx, |ui| { /* … */ });
/// ```
pub fn dialog<'a>(title: impl Into<WidgetText> + 'a, theme: &Theme) -> Window<'a> {
    Window::new(title)
        .collapsible(false)
        .resizable(true)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(theme.card_frame())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_chrome_bottom_sums_nav_and_ime() {
        let c = SystemChrome {
            top: 36.0,
            nav_bottom: 20.0,
            ime_bottom: 400.0,
        };
        assert!((c.bottom() - 420.0).abs() < f32::EPSILON);
        assert!(!c.is_zero());
    }

    #[test]
    fn from_insets_zeroes_ime() {
        let c = SystemChrome::from_insets(36.0, 20.0);
        assert_eq!(c.ime_bottom, 0.0);
        assert_eq!(c.nav_bottom, 20.0);
    }
}
