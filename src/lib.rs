//! **Vidya** — theme layer on top of [egui].
//!
//! Palette and spacing track GNOME HIG / libadwaita *feel* without linking GTK
//! or using that name in the public API. Dark is the default.
//!
//! On Android (edge-to-edge), call [`reserve_system_chrome`] once per frame
//! **before** other panels, or use [`top_header`] so content cannot sit under
//! the system status / navigation bars.
//!
//! ```ignore
//! use vidya::{apply_dark, Theme};
//!
//! apply_dark(ctx);
//! if vidya::primary_button(ui, &Theme::dark(), "Open").clicked() { /* … */ }
//! ```

mod chrome;
mod theme;

pub use chrome::{reserve_system_chrome, system_chrome, top_header, SystemChrome};
pub use theme::{
    body, button, checkbox, destructive_button, dim_label, primary_button, text_field_multiline,
    text_field_singleline, title, title_2, Mode, Palette, Spacing, Theme, TypeScale,
};

use egui::{Context, FontFamily, FontId, Style, TextStyle};

/// Install palette + spacing + text styles on the egui context.
pub fn apply(ctx: &Context, theme: &Theme) {
    ctx.set_visuals(theme.visuals());
    ctx.set_style(theme.style());
}

/// Dark shell (default).
pub fn apply_dark(ctx: &Context) {
    apply(ctx, &Theme::dark());
}

/// Light shell.
pub fn apply_light(ctx: &Context) {
    apply(ctx, &Theme::light());
}

/// Map type scale onto egui text styles.
pub fn install_text_styles(style: &mut Style, scale: &TypeScale) {
    use FontFamily::Proportional;
    style.text_styles = [
        (TextStyle::Small, FontId::new(scale.caption, Proportional)),
        (TextStyle::Body, FontId::new(scale.body, Proportional)),
        (TextStyle::Button, FontId::new(scale.body, Proportional)),
        (TextStyle::Heading, FontId::new(scale.title, Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(scale.body, FontFamily::Monospace),
        ),
    ]
    .into();
}
