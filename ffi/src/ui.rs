//! The immediate-mode cursor: a stack of live [`egui::Ui`] nodes.
//!
//! The ABI is push/pop (`vidya_card_begin` … `vidya_card_end`) while the Rust
//! API is closure-based (`vidya::card(ui, theme, |ui| …)`). Bridging them means
//! holding a child `Ui` open across FFI calls, which is exactly what
//! [`egui::Frame::begin`] / [`egui::containers::frame::Prepared::end`] and
//! [`egui::Ui::new_child`] are for. Every node is closed into its parent, so
//! layout ends up identical to the closure form.

use egui::containers::frame::Prepared;
use egui::{Align, Align2, Color32, FontId, Id, Layout, Rect, Sense, Ui, UiBuilder, Vec2};
use vidya_core::Theme;

/// A page taller than its viewport scrolls. `egui::ScrollArea` keeps its
/// `begin`/`end` pair private — unlike `egui::Frame`, which is what the card
/// node below borrows — so the page reimplements the half we need: offset the
/// content, clip it to the viewport, and carry the offset between frames.
#[derive(Clone, Copy, Default)]
struct ScrollState {
    /// How far the content is scrolled up, in points. Always >= 0.
    offset: f32,
    /// Flick velocity in points per second, decaying under friction.
    vel: f32,
    /// Content height measured when the page closed last frame.
    content: f32,
}

impl ScrollState {
    fn load(ctx: &egui::Context, id: Id) -> Self {
        ctx.data(|d| d.get_temp(id)).unwrap_or_default()
    }

    fn store(self, ctx: &egui::Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}

/// Pixels per second squared, and the speed below which a flick stops.
const SCROLL_FRICTION: f32 = 1000.0;
const SCROLL_STOP_SPEED: f32 = 20.0;
const SCROLL_BAR_WIDTH: f32 = 6.0;

enum Node {
    /// A plain child region (root, page).
    Region(Ui),
    /// A framed surface (card) that paints its background on close.
    Framed(Box<Prepared>),
    /// A scrolling page: the content Ui plus what closing it needs.
    Scrolled {
        ui: Ui,
        id: Id,
        viewport: Rect,
        state: ScrollState,
    },
}

impl Node {
    fn ui_mut(&mut self) -> &mut Ui {
        match self {
            Self::Region(ui) | Self::Scrolled { ui, .. } => ui,
            Self::Framed(prepared) => &mut prepared.content_ui,
        }
    }
}

/// Open UI nodes for the current frame. Empty between frames.
#[derive(Default)]
pub struct Stack {
    nodes: Vec<Node>,
}

impl Stack {
    pub fn is_active(&self) -> bool {
        !self.nodes.is_empty()
    }

    pub fn top(&mut self) -> Option<&mut Ui> {
        self.nodes.last_mut().map(Node::ui_mut)
    }

    pub fn push_root(&mut self, ctx: &egui::Context) {
        let ui = Ui::new(
            ctx.clone(),
            Id::new("vidya_ffi_root"),
            UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(ctx.screen_rect())
                .layout(Layout::top_down(Align::Min)),
        );
        self.nodes.push(Node::Region(ui));
    }

    /// Vertical page: page padding, optional centred max width, section gaps.
    pub fn push_page(&mut self, theme: &Theme, max_width: f32) {
        let Some(parent) = self.top() else {
            return;
        };
        let pad = theme.spacing.page;
        let mut rect = parent.available_rect_before_wrap().shrink(pad);
        if max_width > 0.0 && rect.width() > max_width {
            let cx = rect.center().x;
            rect = Rect::from_min_max(
                egui::pos2(cx - max_width * 0.5, rect.min.y),
                egui::pos2(cx + max_width * 0.5, rect.max.y),
            );
        }

        let ctx = parent.ctx().clone();
        let id = Id::new("vidya_ffi_page_scroll");
        let mut state = ScrollState::load(&ctx, id);
        // Last frame's height decides whether this frame scrolls at all: the
        // content has not been emitted yet, and an immediate-mode page cannot
        // know its own size in advance.
        let max_offset = (state.content - rect.height()).max(0.0);

        if max_offset > 0.0 {
            // Claim the drag BEFORE any content exists, exactly as egui's own
            // ScrollArea does — a rect interacted after its children would
            // steal their presses instead of scrolling past them.
            let drag = parent.interact(rect, id.with("drag"), Sense::drag());
            let dt = parent.input(|i| i.stable_dt).min(0.1);
            if drag.dragged() {
                state.offset -= parent.input(|i| i.pointer.delta().y);
                state.vel = 0.0;
            } else {
                if drag.drag_stopped() {
                    state.vel = parent.input(|i| i.pointer.velocity().y);
                }
                if state.vel != 0.0 {
                    state.offset -= state.vel * dt;
                    let friction = SCROLL_FRICTION * dt;
                    if friction > state.vel.abs() || state.vel.abs() < SCROLL_STOP_SPEED {
                        state.vel = 0.0;
                    } else {
                        state.vel -= state.vel.signum() * friction;
                    }
                    ctx.request_repaint();
                }
            }
            if parent.rect_contains_pointer(rect) {
                state.offset -= parent.input(|i| i.smooth_scroll_delta.y);
            }
        } else {
            state.vel = 0.0;
        }
        state.offset = state.offset.clamp(0.0, max_offset);

        // The content Ui starts above the viewport by the scroll offset and is
        // free to run as tall as it likes; the clip rect hides the overflow.
        let content = Rect::from_min_size(
            rect.min - Vec2::new(0.0, state.offset),
            egui::vec2(rect.width(), f32::INFINITY),
        );
        let mut ui = parent.new_child(
            UiBuilder::new()
                .max_rect(content)
                .layout(Layout::top_down(Align::Min)),
        );
        ui.set_clip_rect(rect.intersect(parent.clip_rect()));
        ui.spacing_mut().item_spacing.y = theme.spacing.md;
        self.nodes.push(Node::Scrolled {
            ui,
            id,
            viewport: rect,
            state,
        });
    }

    /// Themed card surface. Mirrors `vidya::card`'s width discipline: content
    /// fills the parent column and never overflows past it.
    pub fn push_card(&mut self, theme: &Theme) {
        let Some(parent) = self.top() else {
            return;
        };
        let outer = parent.available_width().max(1.0);
        parent.set_max_width(outer);

        let mut prepared = theme.card_frame().begin(parent);
        let inner = prepared.content_ui.available_width().max(1.0);
        prepared.content_ui.set_min_width(inner);
        prepared.content_ui.set_max_width(inner);
        prepared.content_ui.spacing_mut().item_spacing.y = theme.spacing.sm;
        self.nodes.push(Node::Framed(Box::new(prepared)));
    }

    /// Close the innermost node into its parent. The root is kept — only
    /// [`Self::unwind`] retires it — so a stray `vidya_page_end` cannot leave
    /// the frame without a cursor.
    pub fn pop(&mut self) {
        if self.nodes.len() > 1 {
            self.close_one();
        }
    }

    /// Close every open node, innermost first.
    pub fn unwind(&mut self) {
        while !self.nodes.is_empty() {
            self.close_one();
        }
    }

    fn close_one(&mut self) {
        let Some(node) = self.nodes.pop() else {
            return;
        };
        let Some(parent) = self.nodes.last_mut().map(Node::ui_mut) else {
            return; // Root: nothing to fold into.
        };
        match node {
            Node::Region(child) => {
                parent.advance_cursor_after_rect(child.min_rect());
            }
            Node::Scrolled {
                ui,
                id,
                viewport,
                mut state,
            } => {
                state.content = ui.min_rect().height();
                let ctx = parent.ctx().clone();
                state.store(&ctx, id);
                paint_scroll_bar(parent, viewport, state);
                parent.advance_cursor_after_rect(viewport);
            }
            Node::Framed(prepared) => {
                prepared.end(parent);
            }
        }
    }
}

// ── Leaves ──────────────────────────────────────────────────────────────────

pub fn gap(ui: &mut Ui, pixels: f32) {
    ui.add_space(pixels);
}

pub fn separator(ui: &mut Ui) {
    ui.separator();
}

pub fn status(ui: &mut Ui, theme: &Theme, label: &str, live: bool) {
    ui.horizontal(|ui| {
        vidya_core::status_dot(ui, theme, live);
        vidya_core::body(ui, theme, label);
    });
}

/// Single-line field over a caller-owned buffer.
///
/// The whole [`egui::Response`] is returned rather than just "did it change":
/// the tree backend also needs `lost_focus` to tell Enter from a click
/// elsewhere. Callers that only want the change flag ask it for `.changed()`.
pub fn text_field(
    ui: &mut Ui,
    theme: &Theme,
    text: &mut String,
    placeholder: &str,
) -> egui::Response {
    let response = vidya_core::text_field_singleline(ui, theme, text);
    if text.is_empty() && !placeholder.is_empty() {
        // `text_field_singleline` has no hint-text parameter; paint one in the
        // field's own padding so the ABI's `placeholder` still means something.
        let rect = response.rect;
        ui.painter().text(
            egui::pos2(rect.left() + theme.spacing.field_pad_x, rect.center().y),
            Align2::LEFT_CENTER,
            placeholder,
            FontId::proportional(theme.type_scale.body),
            theme.palette.text_secondary,
        );
    }
    response
}

/// Checkbox over a caller-owned value. Returns the value after input.
pub fn checkbox(ui: &mut Ui, theme: &Theme, checked: bool, label: &str) -> (bool, bool) {
    let mut value = checked;
    let response = vidya_core::checkbox(ui, theme, &mut value, label);
    (value, response.changed())
}

/// Button kinds, matching `VidyaButtonKind`.
pub fn button(ui: &mut Ui, theme: &Theme, label: &str, kind: i32) -> bool {
    match kind {
        1 => vidya_core::primary_button(ui, theme, label),
        2 => vidya_core::destructive_button(ui, theme, label),
        _ => vidya_core::button(ui, theme, label),
    }
    .clicked()
}

/// A thin overlay bar on the viewport's right edge, drawn only when the page
/// actually overflows. egui paints its own bars from private code, so this is
/// the one piece of `ScrollArea` that has to be redrawn by hand.
fn paint_scroll_bar(ui: &Ui, viewport: Rect, state: ScrollState) {
    let max_offset = (state.content - viewport.height()).max(0.0);
    if max_offset <= 0.0 {
        return;
    }
    let visible = (viewport.height() / state.content).clamp(0.0, 1.0);
    let height = (viewport.height() * visible).max(24.0);
    let travel = viewport.height() - height;
    let top = viewport.min.y + travel * (state.offset / max_offset).clamp(0.0, 1.0);
    let bar = Rect::from_min_size(
        egui::pos2(viewport.max.x - SCROLL_BAR_WIDTH, top),
        egui::vec2(SCROLL_BAR_WIDTH, height),
    );
    ui.painter().rect_filled(
        bar,
        SCROLL_BAR_WIDTH * 0.5,
        Color32::from_rgba_unmultiplied(94, 92, 100, 136),
    );
}
