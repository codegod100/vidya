//! Vidya aesthetic showcase — run with:
//!
//! ```bash
//! cargo run --example demo
//! ```

use eframe::egui::{self, Align, Color32, Layout, RichText, ScrollArea, Sense, Stroke, Vec2};
use vidya::{
    apply, body, button, destructive_button, dim_label, primary_button, title, title_2, Mode,
    Theme,
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([920.0, 680.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("Vidya — Aesthetic Showcase"),
        ..Default::default()
    };
    eframe::run_native(
        "Vidya Showcase",
        options,
        Box::new(|cc| Ok(Box::new(DemoApp::new(cc)))),
    )
}

struct DemoApp {
    mode: Mode,
    name: String,
    notes: String,
    enable_sync: bool,
    volume: f32,
    selected_nav: Nav,
    toast: Option<String>,
    click_count: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Nav {
    Overview,
    Typography,
    Actions,
    Surfaces,
    Palette,
    Forms,
}

impl Nav {
    const ALL: [Nav; 6] = [
        Nav::Overview,
        Nav::Typography,
        Nav::Actions,
        Nav::Surfaces,
        Nav::Palette,
        Nav::Forms,
    ];

    fn label(self) -> &'static str {
        match self {
            Nav::Overview => "Overview",
            Nav::Typography => "Typography",
            Nav::Actions => "Actions",
            Nav::Surfaces => "Surfaces",
            Nav::Palette => "Palette",
            Nav::Forms => "Forms",
        }
    }
}

impl DemoApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply(&cc.egui_ctx, &Theme::dark());
        Self {
            mode: Mode::Dark,
            name: String::from("Ada"),
            notes: String::from("A short note about the theme layer."),
            enable_sync: true,
            volume: 0.62,
            selected_nav: Nav::Overview,
            toast: None,
            click_count: 0,
        }
    }

    fn theme(&self) -> Theme {
        match self.mode {
            Mode::Dark => Theme::dark(),
            Mode::Light => Theme::light(),
        }
    }

    fn set_mode(&mut self, ctx: &egui::Context, mode: Mode) {
        self.mode = mode;
        apply(ctx, &self.theme());
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let th = self.theme();
        let p = &th.palette;
        let sp = &th.spacing;

        // ── Headerbar ──────────────────────────────────────────────
        egui::TopBottomPanel::top("header")
            .frame(th.header_frame())
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        title(ui, &th, "Vidya");
                        dim_label(ui, &th, "GNOME/HIG-inspired theme layer for egui");
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let is_dark = self.mode == Mode::Dark;
                        let label = if is_dark { " Light " } else { " Dark " };
                        if button(ui, &th, label).clicked() {
                            let next = if is_dark { Mode::Light } else { Mode::Dark };
                            self.set_mode(ctx, next);
                        }

                        ui.add_space(sp.sm);
                        dim_label(ui, &th, "shell");
                    });
                });
            });

        // ── Toast strip ────────────────────────────────────────────
        if let Some(msg) = self.toast.clone() {
            egui::TopBottomPanel::bottom("toast")
                .frame(
                    egui::Frame::new()
                        .fill(p.accent.gamma_multiply(0.2))
                        .inner_margin(egui::Margin::symmetric(sp.page as i8, sp.sm as i8))
                        .stroke(Stroke::new(1.0, p.accent.gamma_multiply(0.5))),
                )
                .show_separator_line(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        body(ui, &th, &msg);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if button(ui, &th, "Dismiss").clicked() {
                                self.toast = None;
                            }
                        });
                    });
                });
        }

        // ── Side nav ───────────────────────────────────────────────
        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(180.0)
            .frame(
                egui::Frame::new()
                    .fill(p.view_bg)
                    .inner_margin(egui::Margin::same(sp.md as i8))
                    .stroke(Stroke::new(1.0, p.border_soft)),
            )
            .show(ctx, |ui| {
                dim_label(ui, &th, "SECTIONS");
                ui.add_space(sp.sm);

                for nav in Nav::ALL {
                    let selected = self.selected_nav == nav;
                    let fill = if selected {
                        p.accent.gamma_multiply(0.25)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let text_color = if selected { p.accent } else { p.text };

                    let resp = egui::Frame::new()
                        .fill(fill)
                        .corner_radius(sp.radius_md)
                        .inner_margin(egui::Margin::symmetric(sp.md as i8, sp.sm as i8))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.label(
                                RichText::new(nav.label())
                                    .size(th.type_scale.body)
                                    .color(text_color)
                                    .strong_if(selected),
                            );
                        })
                        .response
                        .interact(Sense::click());

                    if resp.hovered() && !selected {
                        ui.painter().rect_filled(
                            resp.rect,
                            sp.radius_md,
                            p.button_hover.gamma_multiply(0.5),
                        );
                    }
                    if resp.clicked() {
                        self.selected_nav = nav;
                    }
                    ui.add_space(sp.xs);
                }

                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    dim_label(ui, &th, "v0.1 · MIT");
                    ui.add_space(sp.sm);
                    dim_label(ui, &th, "nandi.uk/vidya");
                });
            });

        // ── Main content ───────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(th.page_frame())
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(720.0);
                        match self.selected_nav {
                            Nav::Overview => self.ui_overview(ui, &th),
                            Nav::Typography => self.ui_typography(ui, &th),
                            Nav::Actions => self.ui_actions(ui, &th),
                            Nav::Surfaces => self.ui_surfaces(ui, &th),
                            Nav::Palette => self.ui_palette(ui, &th),
                            Nav::Forms => self.ui_forms(ui, &th),
                        }
                    });
            });
    }
}

impl DemoApp {
    fn section_header(&self, ui: &mut egui::Ui, th: &Theme, heading: &str, blurb: &str) {
        title(ui, th, heading);
        ui.add_space(th.spacing.xs);
        dim_label(ui, th, blurb);
        ui.add_space(th.spacing.lg);
    }

    fn ui_overview(&mut self, ui: &mut egui::Ui, th: &Theme) {
        self.section_header(
            ui,
            th,
            "Overview",
            "A calm, high-contrast shell with rounded chrome, soft borders, and a clear accent.",
        );

        th.card_frame().show(ui, |ui| {
            title_2(ui, th, "What is Vidya?");
            ui.add_space(th.spacing.sm);
            body(
                ui,
                th,
                "Vidya is a theme layer for egui — palette, spacing, type scale, and a handful of \
                 styled widgets. It tracks the feel of modern desktop HIG without linking GTK.",
            );
            ui.add_space(th.spacing.md);
            ui.horizontal(|ui| {
                if primary_button(ui, th, "Get started").clicked() {
                    self.selected_nav = Nav::Typography;
                    self.toast = Some("Jump to Typography →".into());
                }
                if button(ui, th, "View palette").clicked() {
                    self.selected_nav = Nav::Palette;
                }
            });
        });

        ui.add_space(th.spacing.md);

        ui.horizontal(|ui| {
            ui.set_min_width(ui.available_width());
            let half = (ui.available_width() - th.spacing.md) / 2.0;

            th.card_frame().show(ui, |ui| {
                ui.set_min_width(half - 4.0);
                title_2(ui, th, "Dark by default");
                ui.add_space(th.spacing.sm);
                body(
                    ui,
                    th,
                    "Window, header, cards, and popovers use a layered charcoal stack with a blue accent.",
                );
            });

            ui.add_space(th.spacing.md);

            th.card_frame().show(ui, |ui| {
                ui.set_min_width(half - 4.0);
                title_2(ui, th, "Light when you want it");
                ui.add_space(th.spacing.sm);
                body(
                    ui,
                    th,
                    "Flip the shell with Theme::light() / apply_light — same metrics, inverted palette.",
                );
            });
        });

        ui.add_space(th.spacing.md);

        th.card_frame().show(ui, |ui| {
            title_2(ui, th, "Design tokens");
            ui.add_space(th.spacing.sm);
            ui.horizontal_wrapped(|ui| {
                token_chip(ui, th, "spacing", "4 · 6 · 12 · 18 · 24");
                token_chip(ui, th, "radius", "6 · 9 · 12");
                token_chip(ui, th, "control", "34px height");
                token_chip(ui, th, "type", "20 / 16 / 14 / 12");
            });
        });
    }

    fn ui_typography(&mut self, ui: &mut egui::Ui, th: &Theme) {
        self.section_header(
            ui,
            th,
            "Typography",
            "A short scale: title, title_2, body, caption — no decorative display faces.",
        );

        th.card_frame().show(ui, |ui| {
            type_row(ui, th, "Title", th.type_scale.title, true);
            ui.add_space(th.spacing.md);
            type_row(ui, th, "Title 2", th.type_scale.title_2, true);
            ui.add_space(th.spacing.md);
            type_row(ui, th, "Body", th.type_scale.body, false);
            ui.add_space(th.spacing.md);
            type_row(ui, th, "Caption / dim", th.type_scale.caption, false);
        });

        ui.add_space(th.spacing.md);

        th.card_frame().show(ui, |ui| {
            title_2(ui, th, "In context");
            ui.add_space(th.spacing.sm);
            title(ui, th, "Preferences");
            dim_label(ui, th, "Appearance · Notifications · Privacy");
            ui.add_space(th.spacing.md);
            body(
                ui,
                th,
                "Secondary copy sits a step quieter so hierarchy stays readable without heavy weight.",
            );
            ui.add_space(th.spacing.sm);
            dim_label(
                ui,
                th,
                "Captions and helper text use text_secondary at caption size.",
            );
        });
    }

    fn ui_actions(&mut self, ui: &mut egui::Ui, th: &Theme) {
        self.section_header(
            ui,
            th,
            "Actions",
            "Primary (accent fill), default (flat), and destructive — all share control height.",
        );

        th.card_frame().show(ui, |ui| {
            title_2(ui, th, "Button row");
            ui.add_space(th.spacing.md);
            ui.horizontal(|ui| {
                if primary_button(ui, th, "Save").clicked() {
                    self.click_count += 1;
                    self.toast = Some(format!("Saved · clicks: {}", self.click_count));
                }
                if button(ui, th, "Cancel").clicked() {
                    self.toast = Some("Cancelled".into());
                }
                if destructive_button(ui, th, "Delete").clicked() {
                    self.toast = Some("Destructive action pressed".into());
                }
            });
            ui.add_space(th.spacing.md);
            dim_label(
                ui,
                th,
                "Suggested actions use accent blue; destructive uses the red token.",
            );
        });

        ui.add_space(th.spacing.md);

        th.card_frame().show(ui, |ui| {
            title_2(ui, th, "Dialog footer");
            ui.add_space(th.spacing.sm);
            body(
                ui,
                th,
                "Typical confirm pattern: dismiss on the left, primary on the right.",
            );
            ui.add_space(th.spacing.lg);
            ui.horizontal(|ui| {
                let _ = button(ui, th, "Back");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let _ = primary_button(ui, th, "Continue");
                    ui.add_space(th.spacing.sm);
                    let _ = button(ui, th, "Not now");
                });
            });
        });

        ui.add_space(th.spacing.md);

        th.card_frame().show(ui, |ui| {
            title_2(ui, th, "Status colors");
            ui.add_space(th.spacing.md);
            ui.horizontal(|ui| {
                status_pill(ui, th, "Success", th.palette.success);
                status_pill(ui, th, "Warning", th.palette.warning);
                status_pill(ui, th, "Error", th.palette.destructive);
                status_pill(ui, th, "Accent", th.palette.accent);
            });
        });
    }

    fn ui_surfaces(&mut self, ui: &mut egui::Ui, th: &Theme) {
        self.section_header(
            ui,
            th,
            "Surfaces",
            "Window → view → card → popover layers, with soft 1px borders and gentle radius.",
        );

        // Layer stack visualization
        th.card_frame().show(ui, |ui| {
            title_2(ui, th, "Layer stack");
            ui.add_space(th.spacing.md);

            let layers = [
                ("window_bg", th.palette.window_bg),
                ("view_bg", th.palette.view_bg),
                ("headerbar / card", th.palette.card_bg),
                ("popover_bg", th.palette.popover_bg),
            ];

            for (i, (name, color)) in layers.iter().enumerate() {
                let inset = (i as f32) * th.spacing.md;
                egui::Frame::new()
                    .fill(*color)
                    .stroke(Stroke::new(1.0, th.palette.border_soft))
                    .corner_radius(th.spacing.radius_md)
                    .inner_margin(egui::Margin::same(th.spacing.md as i8))
                    .show(ui, |ui| {
                        ui.add_space(inset.min(1.0)); // keep structure
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(*name)
                                    .size(th.type_scale.body)
                                    .color(th.palette.text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                dim_label(ui, th, &format_hex(*color));
                            });
                        });
                    });
                ui.add_space(th.spacing.sm);
            }
        });

        ui.add_space(th.spacing.md);

        ui.horizontal(|ui| {
            let w = (ui.available_width() - th.spacing.md) / 2.0;
            th.card_frame().show(ui, |ui| {
                ui.set_width(w - 4.0);
                title_2(ui, th, "Card");
                ui.add_space(th.spacing.sm);
                body(ui, th, "card_frame() — raised content blocks.");
                ui.add_space(th.spacing.md);
                dim_label(ui, th, "radius_lg · soft border");
            });
            ui.add_space(th.spacing.md);
            th.header_frame().show(ui, |ui| {
                ui.set_width(w - 4.0);
                title_2(ui, th, "Header");
                ui.add_space(th.spacing.sm);
                body(ui, th, "header_frame() — top chrome strip.");
                ui.add_space(th.spacing.md);
                dim_label(ui, th, "bottom border · horizontal padding");
            });
        });
    }

    fn ui_palette(&mut self, ui: &mut egui::Ui, th: &Theme) {
        self.section_header(
            ui,
            th,
            "Palette",
            "Semantic tokens — not a flat material list. Swatches update with the shell toggle.",
        );

        let groups: &[(&str, &[(&str, Color32)])] = &[
            (
                "Surfaces",
                &[
                    ("window", th.palette.window_bg),
                    ("view", th.palette.view_bg),
                    ("header", th.palette.headerbar_bg),
                    ("card", th.palette.card_bg),
                    ("popover", th.palette.popover_bg),
                ],
            ),
            (
                "Accent",
                &[
                    ("accent", th.palette.accent),
                    ("hover", th.palette.accent_hover),
                    ("active", th.palette.accent_active),
                    ("on accent", th.palette.accent_fg),
                ],
            ),
            (
                "Feedback",
                &[
                    ("destructive", th.palette.destructive),
                    ("success", th.palette.success),
                    ("warning", th.palette.warning),
                ],
            ),
            (
                "Text",
                &[
                    ("primary", th.palette.text),
                    ("secondary", th.palette.text_secondary),
                    ("disabled", th.palette.text_disabled),
                ],
            ),
            (
                "Controls",
                &[
                    ("button", th.palette.button_bg),
                    ("hover", th.palette.button_hover),
                    ("active", th.palette.button_active),
                    ("border", th.palette.border),
                    ("border soft", th.palette.border_soft),
                ],
            ),
        ];

        for (group, swatches) in groups {
            th.card_frame().show(ui, |ui| {
                title_2(ui, th, group);
                ui.add_space(th.spacing.md);
                ui.horizontal_wrapped(|ui| {
                    for (name, color) in *swatches {
                        swatch(ui, th, name, *color);
                    }
                });
            });
            ui.add_space(th.spacing.md);
        }
    }

    fn ui_forms(&mut self, ui: &mut egui::Ui, th: &Theme) {
        self.section_header(
            ui,
            th,
            "Forms",
            "Native egui widgets inherit Visuals/Style from apply() — no custom paint required.",
        );

        th.card_frame().show(ui, |ui| {
            title_2(ui, th, "Profile");
            ui.add_space(th.spacing.md);

            ui.horizontal(|ui| {
                ui.set_min_width(100.0);
                body(ui, th, "Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.name)
                        .desired_width(220.0)
                        .hint_text("Your name"),
                );
            });
            ui.add_space(th.spacing.md);

            body(ui, th, "Notes");
            ui.add_space(th.spacing.xs);
            ui.add(
                egui::TextEdit::multiline(&mut self.notes)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3),
            );
            ui.add_space(th.spacing.md);

            ui.checkbox(&mut self.enable_sync, "Sync preferences across devices");
            ui.add_space(th.spacing.md);

            ui.horizontal(|ui| {
                body(ui, th, "Volume");
                ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).show_value(true));
            });

            ui.add_space(th.spacing.lg);
            ui.horizontal(|ui| {
                if primary_button(ui, th, "Apply").clicked() {
                    self.toast = Some(format!(
                        "Applied for {} · sync={} · vol={:.0}%",
                        self.name,
                        self.enable_sync,
                        self.volume * 100.0
                    ));
                }
                if button(ui, th, "Reset").clicked() {
                    self.name = "Ada".into();
                    self.notes = "A short note about the theme layer.".into();
                    self.enable_sync = true;
                    self.volume = 0.62;
                    self.toast = Some("Form reset".into());
                }
            });
        });

        ui.add_space(th.spacing.md);

        th.card_frame().show(ui, |ui| {
            title_2(ui, th, "Combo & progress");
            ui.add_space(th.spacing.md);
            egui::ComboBox::from_id_salt("demo_combo")
                .selected_text(self.selected_nav.label())
                .show_ui(ui, |ui| {
                    for nav in Nav::ALL {
                        ui.selectable_value(&mut self.selected_nav, nav, nav.label());
                    }
                });
            ui.add_space(th.spacing.md);
            ui.add(egui::ProgressBar::new(self.volume).text(format!("{:.0}%", self.volume * 100.0)));
        });
    }
}

// ── helpers ────────────────────────────────────────────────────────

fn token_chip(ui: &mut egui::Ui, th: &Theme, key: &str, value: &str) {
    egui::Frame::new()
        .fill(th.palette.popover_bg)
        .stroke(Stroke::new(1.0, th.palette.border_soft))
        .corner_radius(th.spacing.radius_sm)
        .inner_margin(egui::Margin::symmetric(th.spacing.md as i8, th.spacing.sm as i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(key)
                        .size(th.type_scale.caption)
                        .color(th.palette.text_secondary),
                );
                ui.label(
                    RichText::new(value)
                        .size(th.type_scale.caption)
                        .color(th.palette.text)
                        .strong(),
                );
            });
        });
}

fn type_row(ui: &mut egui::Ui, th: &Theme, role: &str, size: f32, strong: bool) {
    ui.horizontal(|ui| {
        ui.set_min_width(120.0);
        dim_label(ui, th, role);
        let mut rt = RichText::new(format!("The quick brown fox — {size:.0}px"))
            .size(size)
            .color(th.palette.text);
        if strong {
            rt = rt.strong();
        }
        ui.label(rt);
    });
}

fn status_pill(ui: &mut egui::Ui, th: &Theme, label: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.2))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.6)))
        .corner_radius(th.spacing.radius_sm)
        .inner_margin(egui::Margin::symmetric(th.spacing.md as i8, th.spacing.sm as i8))
        .show(ui, |ui| {
            ui.label(
                RichText::new(label)
                    .size(th.type_scale.caption)
                    .color(color)
                    .strong(),
            );
        });
}

fn swatch(ui: &mut egui::Ui, th: &Theme, name: &str, color: Color32) {
    ui.vertical(|ui| {
        let size = Vec2::new(92.0, 72.0);
        let (rect, _resp) = ui.allocate_exact_size(size, Sense::hover());

        ui.painter()
            .rect_filled(rect, th.spacing.radius_md, color);
        ui.painter().rect_stroke(
            rect,
            th.spacing.radius_md,
            Stroke::new(1.0, th.palette.border_soft),
            egui::StrokeKind::Outside,
        );

        ui.add_space(4.0);
        ui.label(
            RichText::new(name)
                .size(th.type_scale.caption)
                .color(th.palette.text),
        );
        ui.label(
            RichText::new(format_hex(color))
                .size(th.type_scale.caption)
                .color(th.palette.text_secondary),
        );
    });
}

fn format_hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

trait StrongIf {
    fn strong_if(self, on: bool) -> Self;
}

impl StrongIf for RichText {
    fn strong_if(self, on: bool) -> Self {
        if on {
            self.strong()
        } else {
            self
        }
    }
}
