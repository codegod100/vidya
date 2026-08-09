//! Whole-window Gleam app: Vidya only themes + materializes opcodes.

use eframe::egui;
use vidya::{apply_dark, central_page, dim_label, Theme};

use crate::gleam_bridge::{self, paint_shell_view};

/// Desktop entry where Gleam owns the entire UI tree for the window.
///
/// Requires [`gleam_bridge::install_gleam_shell`] beforehand.
pub fn run_gleam_app() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 560.0])
            .with_min_inner_size([360.0, 480.0])
            .with_title("Gleam App — Vidya shell"),
        ..Default::default()
    };
    eframe::run_native(
        "Gleam App",
        options,
        Box::new(|cc| Ok(Box::new(GleamApp::new(cc)))),
    )
}

struct GleamApp {
    theme: Theme,
    model: Option<i64>,
    err: Option<String>,
}

impl GleamApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_dark(&cc.egui_ctx);
        let theme = Theme::dark();
        let (model, err) = match gleam_bridge::shell_hooks() {
            Some(h) => match (h.init)() {
                Ok(n) => (Some(n), None),
                Err(e) => (None, Some(e)),
            },
            None => (
                None,
                Some("gleam shell hooks not installed (host must call install_gleam_shell)".into()),
            ),
        };
        Self { theme, model, err }
    }
}

impl eframe::App for GleamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_dark(ctx);
        let th = self.theme.clone();

        central_page(ctx, &th, "gleam_app", |g| {
            g.section(|ui| {
                let Some(hooks) = gleam_bridge::shell_hooks() else {
                    dim_label(ui, &th, "No Gleam shell hooks.");
                    return;
                };
                let Some(model) = self.model else {
                    if let Some(err) = &self.err {
                        dim_label(ui, &th, err);
                    }
                    return;
                };

                let painted = paint_shell_view(ui, &th, hooks, model);
                if let Some(err) = painted.error {
                    self.err = Some(err);
                }
                if let Some(msg) = painted.pending_msg {
                    match (hooks.update)(model, msg) {
                        Ok(n) => {
                            self.model = Some(n);
                            self.err = None;
                        }
                        Err(e) => self.err = Some(e),
                    }
                }
                if let Some(err) = &self.err {
                    ui.add_space(th.spacing.sm);
                    dim_label(ui, &th, err);
                }
            });
        });
    }
}
