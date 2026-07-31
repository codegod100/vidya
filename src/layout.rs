//! Layout composition surface — width-safe, compact-by-default primitives.
//!
//! Apps should prefer these helpers over raw `set_max_width` / `Layout::…`
//! plumbing. Escape hatches stay (egui is still available); this module makes
//! the **good defaults** the short path.
//!
//! # Patterns → primitives
//!
//! | App footgun | Primitive |
//! |-------------|-----------|
//! | Page scrolls off / under edge | [`page_body`] / [`central_page`] |
//! | Card overflows column | [`card`] / [`compact_card`] |
//! | Giant gaps when parent is tall | [`vstack`] (non-justified) |
//! | Fixed tiles stretch across the window | [`pack`] (wrap, hug content) |
//! | Actions clipped off the right | [`lead_trail`] |
//! | Side-by-side vs stack breakpoint | [`two_col`] / [`side_by_side`] |
//! | Rate columns staircase ("waterfall") | [`metric_bps`] / [`metric_rate`] / [`metric_cell`] / [`grid_cols`] / [`data_table`] |

use std::hash::Hash;

use egui::{
    Align, FontId, Frame, Grid, InnerResponse, Layout, Margin, RichText, ScrollArea, Sense, Stroke,
    Ui, Vec2,
};

use crate::Theme;

// ── Pure policy (unit-tested without a window) ──────────────────────────────

/// Minimum residual width (px) before two equal columns fit with `gap`.
///
/// `true` means place side-by-side; `false` means stack vertically.
pub fn side_by_side(avail: f32, min_col: f32, gap: f32) -> bool {
    avail >= min_col * 2.0 + gap && min_col > 0.0 && avail > 0.0
}

/// Default min column width for [`two_col`] when apps pass theme spacing.
pub fn default_min_col(theme: &Theme) -> f32 {
    // Roughly one compact card: control + padding.
    theme.spacing.control_height * 6.0 + theme.spacing.page
}

/// Character width of a fixed monospace throughput cell ([`metric_bps`]).
pub const METRIC_BPS_CHARS: usize = 14;

/// Character width of a fixed monospace event-rate cell ([`metric_rate`]).
pub const METRIC_RATE_CHARS: usize = 10;

/// Pixel width for a metric cell painted with caption monospace + padding.
pub fn metric_cell_px(theme: &Theme, chars: usize) -> f32 {
    // Approximate monospace advance ≈ 0.6 × font size; pad for cell breathing room.
    let advance = theme.type_scale.caption * 0.62;
    advance * chars as f32 + theme.spacing.sm * 2.0
}

/// Left-pad `s` to exactly `width` characters (Unicode scalar count).
///
/// Longer strings are returned unchanged (width is a minimum for alignment).
pub fn pad_metric(s: &str, width: usize) -> String {
    let n = s.chars().count();
    if n >= width {
        s.to_string()
    } else {
        format!("{:>width$}", s, width = width)
    }
}

/// Human-readable throughput (B/s → KiB/s, …) without padding.
pub fn format_bps(bps: f64) -> String {
    const UNITS: [&str; 5] = ["B/s", "KiB/s", "MiB/s", "GiB/s", "TiB/s"];
    let mut v = bps.max(0.0);
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{v:.0} {}", UNITS[i])
    } else if v >= 100.0 {
        format!("{v:.0} {}", UNITS[i])
    } else if v >= 10.0 {
        format!("{v:.1} {}", UNITS[i])
    } else {
        format!("{v:.2} {}", UNITS[i])
    }
}

/// Fixed-width throughput for tables (monospace-safe; both edges flush).
pub fn metric_bps(bps: f64) -> String {
    pad_metric(&format_bps(bps), METRIC_BPS_CHARS)
}

/// Human-readable event rate (e.g. write syscalls/s) without padding.
pub fn format_rate(rate: f64) -> String {
    if rate < 0.05 {
        "0/s".into()
    } else if rate < 10.0 {
        format!("{rate:.1}/s")
    } else if rate < 1000.0 {
        format!("{rate:.0}/s")
    } else if rate < 1_000_000.0 {
        format!("{:.1}k/s", rate / 1000.0)
    } else {
        format!("{:.1}M/s", rate / 1_000_000.0)
    }
}

/// Fixed-width event rate for tables.
pub fn metric_rate(rate: f64) -> String {
    pad_metric(&format_rate(rate), METRIC_RATE_CHARS)
}

// ── Width-safe scopes ───────────────────────────────────────────────────────

/// Pin this scope so children cannot expand past the current available width.
pub fn fit_width(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> InnerResponse<()> {
    let w = ui.available_width().max(1.0);
    ui.scope(|ui| {
        ui.set_max_width(w);
        add(ui);
    })
}

/// Same as [`fit_width`], but also set `min_width` so framed children fill the residual.
pub fn fill_width(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> InnerResponse<()> {
    let w = ui.available_width().max(1.0);
    ui.scope(|ui| {
        ui.set_min_width(w);
        ui.set_max_width(w);
        add(ui);
    })
}

/// Non-justified vertical stack — **no giant gaps** when the parent is taller
/// than the content (the usual egui “waterfall” inside stretched cards).
pub fn vstack(ui: &mut Ui, theme: &Theme, add: impl FnOnce(&mut Ui)) -> InnerResponse<()> {
    let gap = theme.spacing.sm;
    ui.with_layout(Layout::top_down(Align::Min), |ui| {
        ui.spacing_mut().item_spacing = Vec2::new(ui.spacing().item_spacing.x, gap);
        add(ui);
    })
}

// ── Surfaces ────────────────────────────────────────────────────────────────

/// Themed card that **fills** the parent column without overflowing past it.
///
/// Content is stacked with [`vstack`] so tall parents do not justify gaps.
pub fn card(ui: &mut Ui, theme: &Theme, add: impl FnOnce(&mut Ui)) -> InnerResponse<()> {
    let outer = ui.available_width().max(1.0);
    ui.set_max_width(outer);
    theme.card_frame().show(ui, |ui| {
        let inner = ui.available_width().max(1.0);
        ui.set_min_width(inner);
        ui.set_max_width(inner);
        vstack(ui, theme, add);
    })
}

/// Compact card with a **fixed outer width** that hugs content height.
///
/// Use for gauge tiles and anomaly panels that must not stretch across the
/// window or absorb leftover horizontal space between siblings.
pub fn compact_card(
    ui: &mut Ui,
    theme: &Theme,
    width: f32,
    add: impl FnOnce(&mut Ui),
) -> InnerResponse<()> {
    let w = width.max(1.0);
    // Reserve a tight horizontal slot so neighbors cannot paint over us.
    ui.allocate_ui_with_layout(
        Vec2::new(w, 0.0),
        Layout::top_down(Align::Min),
        |ui| {
            ui.set_width(w);
            ui.set_max_width(w);
            theme.card_frame().show(ui, |ui| {
                ui.set_width(w - 2.0);
                ui.set_max_width(w - 2.0);
                vstack(ui, theme, add);
            });
        },
    )
}

/// Soft-bordered inset row (popover surface) capped to parent width.
pub fn inset_row(ui: &mut Ui, theme: &Theme, add: impl FnOnce(&mut Ui)) -> InnerResponse<()> {
    let outer = ui.available_width().max(1.0);
    ui.set_max_width(outer);
    Frame::new()
        .fill(theme.palette.popover_bg)
        .stroke(Stroke::new(1.0, theme.palette.border_soft))
        .corner_radius(theme.spacing.radius_sm)
        .inner_margin(Margin::symmetric(
            theme.spacing.md as i8,
            theme.spacing.sm as i8,
        ))
        .show(ui, |ui| {
            let w = ui.available_width().max(1.0);
            ui.set_min_width(w);
            ui.set_max_width(w);
            vstack(ui, theme, add);
        })
}

// ── Horizontal composition ──────────────────────────────────────────────────

/// Horizontal flow that **wraps** before clipping the edge (toolbars / chips).
pub fn hflow(ui: &mut Ui, theme: &Theme, add: impl FnOnce(&mut Ui)) -> InnerResponse<()> {
    let w = ui.available_width().max(1.0);
    let gap = theme.spacing.sm;
    ui.scope(|ui| {
        ui.set_max_width(w);
        ui.spacing_mut().item_spacing = Vec2::new(gap, gap);
        ui.horizontal_wrapped(add);
    })
}

/// Pack of **compact** children (fixed-size cards/tiles) that wrap without
/// stretching leftover horizontal space into empty gaps between items.
///
/// Same wrapping as [`hflow`], but spacing defaults to `md` so packs match
/// gauge / anomaly card groups.
pub fn pack(ui: &mut Ui, theme: &Theme, add: impl FnOnce(&mut Ui)) -> InnerResponse<()> {
    let w = ui.available_width().max(1.0);
    let gap = theme.spacing.md;
    ui.scope(|ui| {
        ui.set_max_width(w);
        ui.spacing_mut().item_spacing = Vec2::new(gap, gap);
        ui.horizontal_wrapped(add);
    })
}

/// Leading content grows into remaining width; trailing actions stay visible.
pub fn lead_trail(
    ui: &mut Ui,
    leading: impl FnOnce(&mut Ui),
    trailing: impl FnOnce(&mut Ui),
) -> InnerResponse<()> {
    let w = ui.available_width().max(1.0);
    ui.scope(|ui| {
        ui.set_max_width(w);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            trailing(ui);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let rest = ui.available_width().max(1.0);
                ui.set_max_width(rest);
                ui.set_min_width(rest.min(ui.available_width()));
                leading(ui);
            });
        });
    })
}

/// Two columns when [`side_by_side`] says so; otherwise stack.
pub fn two_col(
    ui: &mut Ui,
    theme: &Theme,
    min_col: f32,
    left: impl FnOnce(&mut Ui),
    right: impl FnOnce(&mut Ui),
) {
    let gap = theme.spacing.md;
    let avail = ui.available_width();
    if side_by_side(avail, min_col, gap) {
        ui.columns(2, |cols| {
            cols[0].set_width(cols[0].available_width());
            left(&mut cols[0]);
            cols[1].set_width(cols[1].available_width());
            right(&mut cols[1]);
        });
    } else {
        left(ui);
        ui.add_space(gap);
        right(ui);
    }
}

// ── Page shell ──────────────────────────────────────────────────────────────

/// Scrollable page body: pin width to the panel residual, then scroll vertically.
pub fn page_body(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let w = ui.available_width().max(1.0);
    ui.set_max_width(w);
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("vidya_page_body")
        .show(ui, |ui| {
            let inner = ui.available_width().max(1.0);
            ui.set_max_width(inner);
            add(ui);
        });
}

/// Full central page: themed page frame + [`page_body`].
pub fn central_page(
    ctx: &egui::Context,
    theme: &Theme,
    add: impl FnOnce(&mut Ui),
) -> egui::InnerResponse<()> {
    egui::CentralPanel::default()
        .frame(theme.page_frame())
        .show(ctx, |ui| {
            page_body(ui, add);
        })
}

// ── Grid layout DSL ─────────────────────────────────────────────────────────
//
// Declarative rows/columns so apps do not hand-roll `egui::Grid`:
//
// ```ignore
// vidya::grid_cols(ui, &th, "procs", &[
//     ColSpec::Flex,
//     ColSpec::Flex,
//     ColSpec::MetricBps,
//     ColSpec::MetricRate,
// ], |g| {
//     g.row(|r| {
//         r.heading("Name");
//         r.heading("Path");
//         r.heading("Write");
//         r.heading("Write freq");
//     });
//     g.row(|r| {
//         r.text("chrome");
//         r.dim("/usr/bin/chrome");
//         r.metric_bps(write);
//         r.metric_rate(freq);
//     });
// });
// ```

/// Column width hint for the grid DSL.
#[derive(Debug, Clone, Copy)]
pub enum ColSpec {
    /// Grow / shrink with content and leftover space.
    Flex,
    /// Fixed pixel width (e.g. custom metric column).
    Fixed(f32),
    /// Throughput metric column width from theme.
    MetricBps,
    /// Event-rate metric column width from theme.
    MetricRate,
}

impl ColSpec {
    pub fn px(self, theme: &Theme) -> Option<f32> {
        match self {
            ColSpec::Flex => None,
            ColSpec::Fixed(w) => Some(w),
            ColSpec::MetricBps => Some(metric_cell_px(theme, METRIC_BPS_CHARS)),
            ColSpec::MetricRate => Some(metric_cell_px(theme, METRIC_RATE_CHARS)),
        }
    }
}

/// Live grid session (inside `egui::Grid`).
pub struct GridCtx<'ui, 'th> {
    ui: &'ui mut Ui,
    theme: &'th Theme,
    /// Resolved min widths per column (`None` = flex).
    col_widths: Vec<Option<f32>>,
}

/// Grid with explicit column specs (recommended for metric tables).
pub fn grid_cols(
    ui: &mut Ui,
    theme: &Theme,
    id: impl Hash,
    cols: &[ColSpec],
    mut add: impl FnMut(&mut GridCtx<'_, '_>),
) {
    // When no specs are given, pick a high column count so free-form rows fit.
    let n = if cols.is_empty() { 16 } else { cols.len() };
    let col_widths: Vec<Option<f32>> = cols.iter().map(|c| c.px(theme)).collect();
    let spacing = Vec2::new(theme.spacing.md, 2.0);

    Grid::new(id)
        .num_columns(n)
        .spacing(spacing)
        .min_col_width(40.0)
        .striped(true)
        .show(ui, |ui| {
            let mut ctx = GridCtx {
                ui,
                theme,
                col_widths,
            };
            add(&mut ctx);
        });
}

/// Grid with all-flex columns. Prefer [`grid_cols`] when you have metrics.
pub fn grid(
    ui: &mut Ui,
    theme: &Theme,
    id: impl Hash,
    add: impl FnMut(&mut GridCtx<'_, '_>),
) {
    grid_cols(ui, theme, id, &[], add);
}

impl<'ui, 'th> GridCtx<'ui, 'th> {
    /// One table row. Cells are written left→right; `end_row` is automatic.
    pub fn row(&mut self, add: impl FnOnce(&mut RowDsl<'_, 'th>)) {
        let mut col_i = 0usize;
        let mut row = RowDsl {
            ui: self.ui,
            theme: self.theme,
            col_widths: &self.col_widths,
            col_i: &mut col_i,
        };
        add(&mut row);
        self.ui.end_row();
    }

    /// Access the underlying grid `Ui` (escape hatch).
    pub fn ui(&mut self) -> &mut Ui {
        self.ui
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }
}

/// One row inside [`GridCtx::row`].
pub struct RowDsl<'ui, 'th> {
    ui: &'ui mut Ui,
    theme: &'th Theme,
    col_widths: &'ui [Option<f32>],
    col_i: &'ui mut usize,
}

impl<'ui, 'th> RowDsl<'ui, 'th> {
    fn advance(&mut self) {
        *self.col_i += 1;
    }

    fn width_hint(&self) -> Option<f32> {
        self.col_widths.get(*self.col_i).copied().flatten()
    }

    fn metric_width(&self) -> f32 {
        self.width_hint()
            .unwrap_or_else(|| metric_cell_px(self.theme, METRIC_BPS_CHARS))
    }

    /// Free-form cell.
    pub fn cell(&mut self, add: impl FnOnce(&mut Ui)) {
        add(self.ui);
        self.advance();
    }

    /// Column header (caption, strong, secondary). Metric cols right-align.
    pub fn heading(&mut self, text: &str) {
        if let Some(width) = self.width_hint() {
            self.ui.allocate_ui_with_layout(
                Vec2::new(width, self.theme.type_scale.caption + 6.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    ui.set_min_width(width);
                    ui.label(
                        RichText::new(text)
                            .size(self.theme.type_scale.caption)
                            .strong()
                            .color(self.theme.palette.text_secondary),
                    );
                },
            );
        } else {
            self.ui.label(
                RichText::new(text)
                    .size(self.theme.type_scale.caption)
                    .strong()
                    .color(self.theme.palette.text_secondary),
            );
        }
        self.advance();
    }

    /// Primary body text (flex).
    pub fn text(&mut self, text: &str) {
        table_text(self.ui, self.theme, text, true);
        self.advance();
    }

    /// Secondary caption text (flex).
    pub fn dim(&mut self, text: &str) {
        table_text(self.ui, self.theme, text, false);
        self.advance();
    }

    /// Warning-colored strong caption (e.g. anomaly process name).
    pub fn warn(&mut self, text: &str) {
        self.ui.label(
            RichText::new(text)
                .size(self.theme.type_scale.caption)
                .strong()
                .color(self.theme.palette.warning),
        );
        self.advance();
    }

    /// Fixed-width right-aligned monospace metric (`text` from [`metric_bps`] / [`metric_rate`]).
    pub fn metric(&mut self, text: &str) {
        let w = self.metric_width();
        metric_cell(self.ui, self.theme, w, text, false);
        self.advance();
    }

    /// Secondary (dim) metric.
    pub fn metric_dim(&mut self, text: &str) {
        let w = self.metric_width();
        metric_cell(self.ui, self.theme, w, text, true);
        self.advance();
    }

    /// Throughput from raw B/s.
    pub fn metric_bps(&mut self, bps: f64) {
        let w = self
            .width_hint()
            .unwrap_or_else(|| metric_cell_px(self.theme, METRIC_BPS_CHARS));
        metric_cell(self.ui, self.theme, w, &metric_bps(bps), false);
        self.advance();
    }

    /// Event rate from raw 1/s.
    pub fn metric_rate(&mut self, rate: f64) {
        let w = self
            .width_hint()
            .unwrap_or_else(|| metric_cell_px(self.theme, METRIC_RATE_CHARS));
        metric_cell(self.ui, self.theme, w, &metric_rate(rate), true);
        self.advance();
    }
}

// ── Metrics / tables ────────────────────────────────────────────────────────

/// Paint a fixed-width monospace metric string, right-edge aligned in `width` px.
pub fn metric_cell(ui: &mut Ui, theme: &Theme, width: f32, text: &str, secondary: bool) {
    let h = theme.type_scale.caption + 8.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width.max(1.0), h), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let color = if secondary {
        theme.palette.text_secondary
    } else {
        theme.palette.text
    };
    let font = FontId::monospace(theme.type_scale.caption);
    let pos = egui::pos2(rect.right() - theme.spacing.xs.max(2.0), rect.center().y);
    ui.painter()
        .text(pos, egui::Align2::RIGHT_CENTER, text, font, color);
}

/// Column kind for [`data_table`] (thin table helper over the grid DSL).
#[derive(Debug, Clone, Copy)]
pub enum ColKind {
    Flex,
    Metric { width: f32 },
}

/// One column header + kind for [`data_table`].
#[derive(Debug, Clone, Copy)]
pub struct Col {
    pub header: &'static str,
    pub kind: ColKind,
}

/// Striped data table built on [`grid_cols`].
///
/// Prefer the row DSL (`grid_cols` + `g.row`) for new code; this keeps the
/// index-callback shape used by existing consumers.
pub fn data_table(
    ui: &mut Ui,
    theme: &Theme,
    id: impl Hash,
    columns: &[Col],
    mut row: impl FnMut(&mut Ui, usize),
    row_count: usize,
) {
    let specs: Vec<ColSpec> = columns
        .iter()
        .map(|c| match c.kind {
            ColKind::Flex => ColSpec::Flex,
            ColKind::Metric { width } => ColSpec::Fixed(width),
        })
        .collect();

    // Headers via DSL; body rows use the raw grid `Ui` for back-compat callbacks.
    let n = columns.len().max(1);
    Grid::new(id)
        .num_columns(n)
        .spacing([theme.spacing.md, 2.0])
        .min_col_width(40.0)
        .striped(true)
        .show(ui, |ui| {
            let col_widths: Vec<Option<f32>> = specs.iter().map(|c| c.px(theme)).collect();
            let mut col_i = 0usize;
            {
                let mut r = RowDsl {
                    ui,
                    theme,
                    col_widths: &col_widths,
                    col_i: &mut col_i,
                };
                for col in columns {
                    r.heading(col.header);
                }
            }
            ui.end_row();
            for i in 0..row_count {
                row(ui, i);
                ui.end_row();
            }
        });
}

/// Flex text cell for [`data_table`] rows / grid rows.
pub fn table_text(ui: &mut Ui, theme: &Theme, text: &str, primary: bool) {
    let color = if primary {
        theme.palette.text
    } else {
        theme.palette.text_secondary
    };
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .size(if primary {
                    theme.type_scale.body
                } else {
                    theme.type_scale.caption
                })
                .color(color),
        )
        .truncate(),
    );
}

/// Metric cell for [`data_table`] rows (`text` should be [`metric_bps`] / [`metric_rate`]).
pub fn table_metric(ui: &mut Ui, theme: &Theme, width: f32, text: &str, secondary: bool) {
    metric_cell(ui, theme, width, text, secondary);
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Theme;

    #[test]
    fn side_by_side_policy_matches_two_col_breakpoint() {
        let gap = 12.0;
        let min = 160.0;
        assert!(!side_by_side(100.0, min, gap));
        assert!(!side_by_side(min * 2.0 + gap - 1.0, min, gap));
        assert!(side_by_side(min * 2.0 + gap, min, gap));
        assert!(side_by_side(800.0, min, gap));
        assert!(!side_by_side(800.0, 0.0, gap));
    }

    #[test]
    fn metric_bps_fixed_char_width() {
        for bps in [0.0, 100.0, 44.8 * 1024.0, 2.0 * 1024.0 * 1024.0, 999.0] {
            let s = metric_bps(bps);
            assert_eq!(
                s.chars().count(),
                METRIC_BPS_CHARS,
                "metric_bps({bps}) = {s:?}"
            );
            assert!(
                s.ends_with(format_bps(bps).as_str()),
                "padding must preserve value"
            );
        }
    }

    #[test]
    fn metric_rate_fixed_char_width() {
        for rate in [0.0, 1.5, 134.0, 2600.0, 1_500_000.0] {
            let s = metric_rate(rate);
            assert_eq!(
                s.chars().count(),
                METRIC_RATE_CHARS,
                "metric_rate({rate}) = {s:?}"
            );
            assert!(s.contains(format_rate(rate).as_str()) || s.ends_with(format_rate(rate).as_str()));
        }
    }

    #[test]
    fn pad_metric_is_identity_when_already_wide() {
        let long = "123456789012345"; // 15 > 14
        assert_eq!(pad_metric(long, METRIC_BPS_CHARS), long);
    }

    #[test]
    fn metric_cell_px_scales_with_theme_caption() {
        let th = Theme::dark();
        let a = metric_cell_px(&th, METRIC_BPS_CHARS);
        let b = metric_cell_px(&th, METRIC_RATE_CHARS);
        assert!(a > b);
        assert!(a > 40.0);
    }

    #[test]
    fn default_min_col_positive() {
        let th = Theme::dark();
        assert!(default_min_col(&th) > 100.0);
    }

    #[test]
    fn col_spec_resolves_metric_widths() {
        let th = Theme::dark();
        assert!(ColSpec::Flex.px(&th).is_none());
        assert_eq!(ColSpec::Fixed(120.0).px(&th), Some(120.0));
        let bps = ColSpec::MetricBps.px(&th).unwrap();
        let rate = ColSpec::MetricRate.px(&th).unwrap();
        assert!(bps > rate);
        assert!((bps - metric_cell_px(&th, METRIC_BPS_CHARS)).abs() < 0.01);
    }
}
