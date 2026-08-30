//! The immediate-mode cursor: a stack of live [`egui::Ui`] nodes.
//!
//! The ABI is push/pop (`vidya_card_begin` … `vidya_card_end`) while the Rust
//! API is closure-based (`vidya::card(ui, theme, |ui| …)`). Bridging them means
//! holding a child `Ui` open across FFI calls, which is exactly what
//! [`egui::Frame::begin`] / [`egui::containers::frame::Prepared::end`] and
//! [`egui::Ui::new_child`] are for. Every node is closed into its parent, so
//! layout ends up identical to the closure form.

use egui::containers::frame::Prepared;
use egui::{Align, Align2, FontId, Id, Layout, Rect, Ui, UiBuilder};
use vidya_core::Theme;

enum Node {
    /// A plain child region (root, page).
    Region(Ui),
    /// A framed surface (card) that paints its background on close.
    Framed(Box<Prepared>),
}

impl Node {
    fn ui_mut(&mut self) -> &mut Ui {
        match self {
            Self::Region(ui) => ui,
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
        let mut ui = parent.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::top_down(Align::Min)),
        );
        ui.spacing_mut().item_spacing.y = theme.spacing.md;
        self.nodes.push(Node::Region(ui));
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

/// Single-line field over a caller-owned buffer. Returns whether it changed.
pub fn text_field(ui: &mut Ui, theme: &Theme, text: &mut String, placeholder: &str) -> bool {
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
    response.changed()
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
