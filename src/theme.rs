//! Palette, metrics, egui Visuals/Style, and widget helpers.

use egui::{
    style::WidgetVisuals, Button, Color32, CornerRadius, Frame, Margin, Response, RichText, Shadow,
    Stroke, Style, Ui, Vec2, Visuals,
};

use crate::install_text_styles;

/// Light vs dark shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Dark,
    Light,
}

/// Semantic colors.
#[derive(Debug, Clone)]
pub struct Palette {
    pub window_bg: Color32,
    pub view_bg: Color32,
    pub headerbar_bg: Color32,
    pub card_bg: Color32,
    pub popover_bg: Color32,
    pub shade: Color32,

    pub accent: Color32,
    pub accent_fg: Color32,
    pub accent_hover: Color32,
    pub accent_active: Color32,

    pub destructive: Color32,
    pub success: Color32,
    pub warning: Color32,

    pub text: Color32,
    pub text_secondary: Color32,
    pub text_disabled: Color32,

    pub border: Color32,
    pub border_soft: Color32,

    pub button_bg: Color32,
    pub button_hover: Color32,
    pub button_active: Color32,
    pub button_fg: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            window_bg: rgb(0x24, 0x24, 0x24),
            view_bg: rgb(0x1e, 0x1e, 0x1e),
            headerbar_bg: rgb(0x30, 0x30, 0x30),
            card_bg: rgb(0x30, 0x30, 0x30),
            popover_bg: rgb(0x38, 0x38, 0x38),
            shade: Color32::from_black_alpha(80),

            accent: rgb(0x35, 0x84, 0xe4),
            accent_fg: Color32::WHITE,
            accent_hover: rgb(0x4a, 0x93, 0xe7),
            accent_active: rgb(0x1c, 0x71, 0xd8),

            destructive: rgb(0xc0, 0x1c, 0x28),
            success: rgb(0x2e, 0xc2, 0x7e),
            warning: rgb(0xe5, 0xa5, 0x0a),

            text: rgb(0xff, 0xff, 0xff),
            text_secondary: rgb(0x9a, 0x99, 0x96),
            text_disabled: rgb(0x5e, 0x5c, 0x64),

            border: rgb(0x5e, 0x5c, 0x64),
            border_soft: rgb(0x3d, 0x38, 0x46),

            button_bg: rgb(0x3d, 0x3d, 0x3d),
            button_hover: rgb(0x4a, 0x4a, 0x4a),
            button_active: rgb(0x35, 0x35, 0x35),
            button_fg: rgb(0xff, 0xff, 0xff),
        }
    }

    pub fn light() -> Self {
        Self {
            window_bg: rgb(0xfa, 0xfa, 0xfa),
            view_bg: rgb(0xff, 0xff, 0xff),
            headerbar_bg: rgb(0xff, 0xff, 0xff),
            card_bg: rgb(0xff, 0xff, 0xff),
            popover_bg: rgb(0xff, 0xff, 0xff),
            shade: Color32::from_black_alpha(40),

            accent: rgb(0x35, 0x84, 0xe4),
            accent_fg: Color32::WHITE,
            accent_hover: rgb(0x4a, 0x93, 0xe7),
            accent_active: rgb(0x1c, 0x71, 0xd8),

            destructive: rgb(0xc0, 0x1c, 0x28),
            success: rgb(0x26, 0xa2, 0x69),
            warning: rgb(0xe5, 0xa5, 0x0a),

            text: rgb(0x24, 0x1f, 0x31),
            text_secondary: rgb(0x5e, 0x5c, 0x64),
            text_disabled: rgb(0x9a, 0x99, 0x96),

            border: rgb(0xcd, 0xcd, 0xcd),
            border_soft: rgb(0xe0, 0xe0, 0xe0),

            button_bg: rgb(0xe0, 0xe0, 0xe0),
            button_hover: rgb(0xd0, 0xd0, 0xd0),
            button_active: rgb(0xc0, 0xc0, 0xc0),
            button_fg: rgb(0x24, 0x1f, 0x31),
        }
    }
}

/// Spacing scale (HIG-ish: 6 / 12 / 18 / 24).
#[derive(Debug, Clone)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub page: f32,
    pub control_height: f32,
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 6.0,
            md: 12.0,
            lg: 18.0,
            xl: 24.0,
            page: 16.0,
            control_height: 34.0,
            radius_sm: 6.0,
            radius_md: 9.0,
            radius_lg: 12.0,
        }
    }
}

/// Type sizes (UI sans scale).
#[derive(Debug, Clone)]
pub struct TypeScale {
    pub title: f32,
    pub title_2: f32,
    pub title_3: f32,
    pub body: f32,
    pub caption: f32,
}

impl Default for TypeScale {
    fn default() -> Self {
        Self {
            title: 20.0,
            title_2: 16.0,
            title_3: 14.0,
            body: 14.0,
            caption: 12.0,
        }
    }
}

/// Full theme bundle.
#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: Mode,
    pub palette: Palette,
    pub spacing: Spacing,
    pub type_scale: TypeScale,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            mode: Mode::Dark,
            palette: Palette::dark(),
            spacing: Spacing::default(),
            type_scale: TypeScale::default(),
        }
    }

    pub fn light() -> Self {
        Self {
            mode: Mode::Light,
            palette: Palette::light(),
            spacing: Spacing::default(),
            type_scale: TypeScale::default(),
        }
    }

    pub fn visuals(&self) -> Visuals {
        let p = &self.palette;
        let mut v = match self.mode {
            Mode::Dark => Visuals::dark(),
            Mode::Light => Visuals::light(),
        };

        v.dark_mode = self.mode == Mode::Dark;
        v.override_text_color = Some(p.text);
        v.hyperlink_color = p.accent;
        v.warn_fg_color = p.warning;
        v.error_fg_color = p.destructive;
        v.extreme_bg_color = p.view_bg;
        v.faint_bg_color = p.card_bg;
        v.code_bg_color = p.popover_bg;
        v.window_fill = p.window_bg;
        v.panel_fill = p.window_bg;
        v.window_stroke = Stroke::new(1.0, p.border_soft);
        v.window_corner_radius = CornerRadius::same(self.spacing.radius_md as u8);
        v.window_shadow = Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: p.shade,
        };
        v.popup_shadow = Shadow {
            offset: [0, 2],
            blur: 8,
            spread: 0,
            color: p.shade,
        };
        v.selection.bg_fill = p.accent.gamma_multiply(0.35);
        v.selection.stroke = Stroke::new(1.0, p.accent);
        v.widgets = widget_visuals(p, &self.spacing);
        v
    }

    pub fn style(&self) -> Style {
        let mut style = Style {
            visuals: self.visuals(),
            ..Style::default()
        };
        install_text_styles(&mut style, &self.type_scale);

        let sp = &self.spacing;
        style.spacing.item_spacing = Vec2::new(sp.sm, sp.sm);
        style.spacing.button_padding = Vec2::new(sp.md, sp.sm);
        style.spacing.indent = sp.lg;
        style.spacing.window_margin = Margin::same(sp.page as i8);
        style.spacing.menu_margin = Margin::same(sp.sm as i8);
        style.spacing.interact_size = Vec2::new(sp.control_height * 2.5, sp.control_height);
        style.spacing.icon_width = 16.0;
        style.spacing.icon_width_inner = 14.0;
        style.interaction.tooltip_delay = 0.4;
        style.animation_time = 0.12;
        style
    }

    pub fn header_frame(&self) -> Frame {
        Frame::new()
            .fill(self.palette.headerbar_bg)
            .inner_margin(Margin::symmetric(self.spacing.page as i8, self.spacing.md as i8))
            .stroke(Stroke::new(1.0, self.palette.border_soft))
    }

    pub fn card_frame(&self) -> Frame {
        Frame::new()
            .fill(self.palette.card_bg)
            .corner_radius(self.spacing.radius_lg)
            .inner_margin(Margin::same(self.spacing.md as i8))
            .stroke(Stroke::new(1.0, self.palette.border_soft))
    }

    pub fn page_frame(&self) -> Frame {
        Frame::new()
            .fill(self.palette.window_bg)
            .inner_margin(Margin::same(self.spacing.page as i8))
    }
}

fn widget_visuals(p: &Palette, sp: &Spacing) -> egui::style::Widgets {
    let r = CornerRadius::same(sp.radius_md as u8);
    let soft = Stroke::new(1.0, p.border_soft);

    egui::style::Widgets {
        noninteractive: WidgetVisuals {
            bg_fill: p.card_bg,
            weak_bg_fill: p.view_bg,
            bg_stroke: soft,
            corner_radius: r,
            fg_stroke: Stroke::new(1.0, p.text_secondary),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            bg_fill: p.button_bg,
            weak_bg_fill: p.button_bg,
            bg_stroke: Stroke::new(1.0, p.border_soft),
            corner_radius: r,
            fg_stroke: Stroke::new(1.0, p.button_fg),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            bg_fill: p.button_hover,
            weak_bg_fill: p.button_hover,
            bg_stroke: Stroke::new(1.0, p.border),
            corner_radius: r,
            fg_stroke: Stroke::new(1.0, p.text),
            expansion: 0.0,
        },
        active: WidgetVisuals {
            bg_fill: p.button_active,
            weak_bg_fill: p.button_active,
            bg_stroke: Stroke::new(1.0, p.accent),
            corner_radius: r,
            fg_stroke: Stroke::new(1.0, p.text),
            expansion: 0.0,
        },
        open: WidgetVisuals {
            bg_fill: p.popover_bg,
            weak_bg_fill: p.popover_bg,
            bg_stroke: Stroke::new(1.0, p.border),
            corner_radius: r,
            fg_stroke: Stroke::new(1.0, p.text),
            expansion: 0.0,
        },
    }
}

/// Primary / suggested action (accent filled).
pub fn primary_button(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    let p = &theme.palette;
    let text = RichText::new(label)
        .size(theme.type_scale.body)
        .color(p.accent_fg);
    let btn = Button::new(text)
        .fill(p.accent)
        .stroke(Stroke::NONE)
        .corner_radius(theme.spacing.radius_md)
        .min_size(Vec2::new(0.0, theme.spacing.control_height));
    ui.add(btn)
}

/// Regular flat button.
pub fn button(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    let p = &theme.palette;
    let text = RichText::new(label)
        .size(theme.type_scale.body)
        .color(p.button_fg);
    let btn = Button::new(text)
        .fill(p.button_bg)
        .stroke(Stroke::new(1.0, p.border_soft))
        .corner_radius(theme.spacing.radius_md)
        .min_size(Vec2::new(0.0, theme.spacing.control_height));
    ui.add(btn)
}

/// Destructive action.
pub fn destructive_button(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    let p = &theme.palette;
    let text = RichText::new(label)
        .size(theme.type_scale.body)
        .color(Color32::WHITE);
    let btn = Button::new(text)
        .fill(p.destructive)
        .stroke(Stroke::NONE)
        .corner_radius(theme.spacing.radius_md)
        .min_size(Vec2::new(0.0, theme.spacing.control_height));
    ui.add(btn)
}

pub fn title(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .size(theme.type_scale.title)
            .strong()
            .color(theme.palette.text),
    );
}

pub fn title_2(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .size(theme.type_scale.title_2)
            .strong()
            .color(theme.palette.text),
    );
}

pub fn body(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .size(theme.type_scale.body)
            .color(theme.palette.text),
    );
}

pub fn dim_label(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text)
            .size(theme.type_scale.caption)
            .color(theme.palette.text_secondary),
    );
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
