//! A retained node tree, painted immediately.
//!
//! The push/pop half of this ABI (`vidya_card_begin` … `vidya_card_end`) suits
//! a caller that writes its UI out top to bottom every frame. A *reactive*
//! caller does not: glimmer keeps a component tree, reconciles it against new
//! hiccup, and emits create/patch/append/remove against whatever the toolkit
//! calls a widget. GTK has widgets to hand it; egui has none.
//!
//! So this module is the widget layer glimmer expects, on the Rust side of the
//! FFI. The caller gets integer node handles and mutates them — set a prop,
//! append a child, drop a subtree. Nothing is drawn by those calls. Once a
//! frame, [`Tree::paint`] walks the whole tree and emits the egui calls it
//! describes, and interactions come back out as a queue of events the caller
//! drains and routes to its own handlers.
//!
//! Two things fall out of that split that the push/pop ABI could not have:
//!
//! * **Closure-shaped egui APIs work.** `ScrollArea`, `Frame` and friends take
//!   an `FnOnce(&mut Ui)` and keep their `begin`/`end` private, which is why
//!   `vidya_page_begin` had to reimplement scrolling by hand and why the page
//!   was documented as non-scrolling. Painting from a tree we already hold
//!   means the recursion *is* the closure; nothing has to stay open across a
//!   call boundary.
//! * **FFI traffic tracks edits, not frames.** A static UI at 60fps costs zero
//!   crossings per frame; only what the reconciler actually changed is sent.
//!
//! The tree deliberately knows nothing about egui until [`Tree::paint`], so the
//! arena and its edit operations are unit-testable with no window.

use std::collections::HashMap;
use std::collections::VecDeque;

use egui::{Align, Align2, Color32, FontId, Id, Layout, Margin, TextureOptions, Ui, Vec2};
use vidya_core::Theme;

/// A prop value. The three types the ABI can carry, and all glimmer needs:
/// keywords and colours arrive as strings, numbers as doubles, flags as ints.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
}

/// What a node renders as. Unknown tags are kept rather than rejected: they
/// paint as a plain vertical box, so a caller using a tag this backend has not
/// grown yet still sees its children instead of nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tag {
    Window,
    Box,
    Page,
    Card,
    Frame,
    Scroll,
    Label,
    Link,
    Title,
    Title2,
    DimLabel,
    Button,
    CheckButton,
    Entry,
    Separator,
    Spacer,
    Progress,
    Spinner,
    Image,
    Avatar,
    Reaction,
    Status,
    Unknown,
}

impl Tag {
    /// Parse a hiccup tag name. `:hbox`/`:vbox` are the same box — the tag only
    /// implies an orientation, which the caller sets as a prop.
    fn parse(name: &str) -> Self {
        match name {
            "window" => Self::Window,
            "box" | "hbox" | "vbox" => Self::Box,
            "page" => Self::Page,
            "card" => Self::Card,
            "frame" => Self::Frame,
            "scroll" => Self::Scroll,
            "label" => Self::Label,
            "link" => Self::Link,
            "title" => Self::Title,
            "title-2" => Self::Title2,
            "dim-label" => Self::DimLabel,
            "button" => Self::Button,
            "checkbutton" | "checkbox" => Self::CheckButton,
            "entry" => Self::Entry,
            "separator" => Self::Separator,
            "spacer" | "gap" => Self::Spacer,
            "progress" => Self::Progress,
            "spinner" => Self::Spinner,
            "image" => Self::Image,
            "avatar" => Self::Avatar,
            "reaction" => Self::Reaction,
            "status" => Self::Status,
            _ => Self::Unknown,
        }
    }

    /// The canonical name of a parsed tag: `:hbox` and `:vbox` both answer
    /// `box`, since the orientation lives in a prop rather than in the tag.
    fn name(&self) -> &'static str {
        match self {
            Self::Window => "window",
            Self::Box => "box",
            Self::Page => "page",
            Self::Card => "card",
            Self::Frame => "frame",
            Self::Scroll => "scroll",
            Self::Label => "label",
            Self::Link => "link",
            Self::Title => "title",
            Self::Title2 => "title-2",
            Self::DimLabel => "dim-label",
            Self::Button => "button",
            Self::CheckButton => "checkbutton",
            Self::Entry => "entry",
            Self::Separator => "separator",
            Self::Spacer => "spacer",
            Self::Progress => "progress",
            Self::Spinner => "spinner",
            Self::Image => "image",
            Self::Avatar => "avatar",
            Self::Reaction => "reaction",
            Self::Status => "status",
            Self::Unknown => "",
        }
    }
}

/// One interaction, waiting to be drained by the caller.
///
/// Names match glimmer's handler props with the `on-` dropped: `click` pairs
/// with `:on-click`, `change` with `:on-change`, and so on. `text` and `num`
/// carry the payload the handler is called with, empty when it takes none.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub node: u32,
    pub name: &'static str,
    pub text: String,
    pub num: f64,
}

#[derive(Clone, Debug, Default)]
struct Node {
    tag: Tag,
    props: HashMap<String, Value>,
    children: Vec<u32>,
    /// 0 when unparented. The root's parent is 0 too, which is what stops the
    /// ancestor walk in [`Tree::would_cycle`].
    parent: u32,
}

impl Default for Tag {
    fn default() -> Self {
        Self::Unknown
    }
}

/// The node arena.
///
/// Handles are `index + 1`, so 0 is always "no node" — the value C gets back
/// from a failed allocation and the sibling argument that means "first".
/// Freed slots are reused, so a list that churns rows does not grow the arena.
pub struct Tree {
    nodes: Vec<Option<Node>>,
    free: Vec<u32>,
    root: u32,
    /// Decoded images, by the path they came from. An `:image` node is walked
    /// every frame and must not decode a file every time.
    textures: HashMap<String, Option<egui::TextureHandle>>,
    pending: VecDeque<Event>,
    /// The event most recently dequeued by `poll`, whose fields the accessors
    /// read. Held here so the ABI can return a payload without out-parameters.
    current: Option<Event>,
}

/// A stable colour for a name: the same person is the same colour every time,
/// and two people are unlikely to share one. Kept dark enough for the light
/// text drawn on top and dull enough not to compete with the accent.
fn name_colour(name: &str, theme: &Theme) -> Color32 {
    let mut hash: u32 = 2166136261;
    for b in name.as_bytes() {
        hash ^= *b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    // Six hues around the wheel, at a fixed saturation and value, rather than
    // free RGB: random channels give muddy colours as often as good ones.
    let sector = (hash % 6) as f32;
    let (r, g, b) = match sector as u32 {
        0 => (0.80, 0.35, 0.35),
        1 => (0.80, 0.55, 0.25),
        2 => (0.45, 0.65, 0.35),
        3 => (0.30, 0.60, 0.65),
        4 => (0.40, 0.50, 0.80),
        _ => (0.65, 0.40, 0.70),
    };
    let _ = theme;
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

impl Tree {
    /// Whether anything under `id` has `:scroll-here` set this frame.
    ///
    /// Walked rather than remembered: the prop is set for the moment of a jump
    /// and taken off again, so there is nothing to keep, and this runs once
    /// per scroll area rather than once per node.
    fn wants_scroll_to(&self, id: u32) -> bool {
        let Some(node) = self.slot(id) else {
            return false;
        };
        matches!(node.props.get("scroll-here"), Some(Value::Bool(true)))
            || node
                .children
                .iter()
                .any(|child| self.wants_scroll_to(*child))
    }

    /// The texture for a file, decoding it the first time it is asked for.
    /// A file that will not decode is remembered as such, so a bad path costs
    /// one failed read rather than one per frame.
    fn texture(&mut self, ui: &Ui, path: &str) -> Option<egui::TextureHandle> {
        if let Some(cached) = self.textures.get(path) {
            return cached.clone();
        }
        let handle = std::fs::read(path)
            .ok()
            .and_then(|bytes| decode_png_rgba(&bytes))
            .map(|image| {
                ui.ctx()
                    .load_texture(format!("vidya/tree/{path}"), image, TextureOptions::LINEAR)
            });
        self.textures.insert(path.to_owned(), handle.clone());
        handle
    }
}

/// PNG bytes as an egui image. PNG alone: it is what the vendored decoder
/// reads, and what the media this paints is served as.
fn decode_png_rgba(bytes: &[u8]) -> Option<egui::ColorImage> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let raw = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => raw.to_vec(),
        png::ColorType::Rgb => raw
            .chunks_exact(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        _ => return None,
    };
    (rgba.len() == w * h * 4).then(|| egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba))
}

impl Default for Tree {
    fn default() -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            free: Vec::new(),
            root: 0,
            textures: HashMap::new(),
            pending: VecDeque::new(),
            current: None,
        };
        tree.root = tree.new_node("window");
        tree
    }
}

impl Tree {
    pub fn root(&self) -> u32 {
        self.root
    }

    fn slot(&self, id: u32) -> Option<&Node> {
        if id == 0 {
            return None;
        }
        self.nodes.get(id as usize - 1).and_then(Option::as_ref)
    }

    fn slot_mut(&mut self, id: u32) -> Option<&mut Node> {
        if id == 0 {
            return None;
        }
        self.nodes.get_mut(id as usize - 1).and_then(Option::as_mut)
    }

    pub fn exists(&self, id: u32) -> bool {
        self.slot(id).is_some()
    }

    // ── editing ─────────────────────────────────────────────────────────────

    pub fn new_node(&mut self, tag: &str) -> u32 {
        let node = Node {
            tag: Tag::parse(tag),
            ..Node::default()
        };
        match self.free.pop() {
            Some(id) => {
                self.nodes[id as usize - 1] = Some(node);
                id
            }
            None => {
                self.nodes.push(Some(node));
                self.nodes.len() as u32
            }
        }
    }

    /// Drop `id` and everything under it, unparenting it first.
    ///
    /// glimmer has no separate destroy operation — `remove-child!` is the last
    /// the reconciler ever says about a widget — so removal frees, and a node
    /// handle the caller still holds after that is simply dead.
    pub fn free_node(&mut self, id: u32) {
        let parent = match self.slot(id) {
            Some(n) => n.parent,
            None => return,
        };
        self.detach(parent, id);
        self.free_subtree(id);
    }

    fn free_subtree(&mut self, id: u32) {
        let Some(node) = self.slot_mut(id).map(std::mem::take) else {
            return;
        };
        self.nodes[id as usize - 1] = None;
        self.free.push(id);
        for child in node.children {
            self.free_subtree(child);
        }
        // An event queued against a node that has since been removed would be
        // routed to a handler the caller has already forgotten.
        self.pending.retain(|e| e.node != id);
    }

    /// Unparent `child` without freeing it. `parent` may be 0 (already loose).
    fn detach(&mut self, parent: u32, child: u32) {
        if let Some(p) = self.slot_mut(parent) {
            p.children.retain(|&c| c != child);
        }
        if let Some(c) = self.slot_mut(child) {
            c.parent = 0;
        }
    }

    /// True when parenting `child` under `parent` would make a loop — `child`
    /// is `parent`, or an ancestor of it. A cycle here is an infinite paint,
    /// so it is checked rather than trusted.
    fn would_cycle(&self, parent: u32, child: u32) -> bool {
        let mut at = parent;
        while at != 0 {
            if at == child {
                return true;
            }
            at = match self.slot(at) {
                Some(n) => n.parent,
                None => 0,
            };
        }
        false
    }

    pub fn append(&mut self, parent: u32, child: u32) -> bool {
        self.insert_at(parent, child, usize::MAX)
    }

    fn insert_at(&mut self, parent: u32, child: u32, index: usize) -> bool {
        if parent == 0 || child == 0 || !self.exists(parent) || !self.exists(child) {
            return false;
        }
        if self.would_cycle(parent, child) {
            return false;
        }
        // Moving a child that already has a parent (including this one) is a
        // reparent, not a duplicate: take it out first so it appears once.
        let old_parent = self.slot(child).map_or(0, |n| n.parent);
        self.detach(old_parent, child);

        let p = self.slot_mut(parent).expect("checked above");
        let at = index.min(p.children.len());
        p.children.insert(at, child);
        self.slot_mut(child).expect("checked above").parent = parent;
        true
    }

    pub fn remove(&mut self, parent: u32, child: u32) {
        if self.slot(child).map_or(true, |n| n.parent != parent) {
            return;
        }
        self.free_node(child);
    }

    /// Move `child` to sit immediately after `sibling`; `sibling` 0 means first.
    /// glimmer's keyed reconciliation calls this to reorder a list without
    /// rebuilding the widgets in it.
    pub fn insert_after(&mut self, parent: u32, child: u32, sibling: u32) -> bool {
        if !self.exists(parent) || !self.exists(child) {
            return false;
        }
        let index = if sibling == 0 {
            0
        } else {
            match self
                .slot(parent)
                .and_then(|p| p.children.iter().position(|&c| c == sibling))
            {
                // The sibling's own slot, once `child` is out of the way, is
                // the position after it.
                Some(i) => i + 1,
                None => return false,
            }
        };
        // Re-derive the index after detaching: removing `child` from earlier in
        // the list shifts everything after it down one.
        let before = self
            .slot(parent)
            .and_then(|p| p.children.iter().position(|&c| c == child))
            .map_or(false, |i| i < index);
        self.insert_at(parent, child, if before { index - 1 } else { index })
    }

    pub fn replace(&mut self, parent: u32, old: u32, new: u32) -> bool {
        let Some(index) = self
            .slot(parent)
            .and_then(|p| p.children.iter().position(|&c| c == old))
        else {
            return false;
        };
        if !self.insert_at(parent, new, index) {
            return false;
        }
        self.remove(parent, old);
        true
    }

    /// The canonical tag name, or the empty string for a node that is not
    /// there. With [`Tree::child_count`] and [`Tree::child_at`] this is enough
    /// for a caller to read back the tree it built — which is how the jolt
    /// backend's tests assert against a real reconcile with no window open.
    pub fn tag_name(&self, id: u32) -> &'static str {
        self.slot(id).map_or("", |n| n.tag.name())
    }

    pub fn child_count(&self, id: u32) -> usize {
        self.slot(id).map_or(0, |n| n.children.len())
    }

    pub fn child_at(&self, id: u32, index: usize) -> u32 {
        self.slot(id)
            .and_then(|n| n.children.get(index))
            .copied()
            .unwrap_or(0)
    }

    // ── props ───────────────────────────────────────────────────────────────

    pub fn set(&mut self, id: u32, key: &str, value: Value) {
        if let Some(node) = self.slot_mut(id) {
            node.props.insert(key.to_owned(), value);
        }
    }

    pub fn clear_props(&mut self, id: u32) {
        if let Some(node) = self.slot_mut(id) {
            node.props.clear();
        }
    }

    pub fn get(&self, id: u32, key: &str) -> Option<&Value> {
        self.slot(id).and_then(|n| n.props.get(key))
    }

    // ── events ──────────────────────────────────────────────────────────────

    fn emit(&mut self, node: u32, name: &'static str, text: String, num: f64) {
        self.pending.push_back(Event {
            node,
            name,
            text,
            num,
        });
    }

    /// Dequeue one event into the accessor slot. False when the queue is empty.
    pub fn poll(&mut self) -> bool {
        self.current = self.pending.pop_front();
        self.current.is_some()
    }

    pub fn current(&self) -> Option<&Event> {
        self.current.as_ref()
    }

    // ── painting ────────────────────────────────────────────────────────────

    /// Emit the whole tree into `ui`. Called once per frame.
    pub fn paint(&mut self, ui: &mut Ui, theme: &Theme) {
        let root = self.root;
        self.paint_node(root, ui, theme);
    }

    fn paint_children(&mut self, id: u32, ui: &mut Ui, theme: &Theme) {
        // The child list is copied rather than borrowed: painting a child can
        // write a prop back (an entry's text) or queue an event, both of which
        // need `&mut self` while the walk is in flight. A UI's worth of `u32`s
        // is a cheap price for not threading a cell through every widget.
        let children = self
            .slot(id)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        for child in children {
            self.paint_node(child, ui, theme);
        }
    }

    fn paint_node(&mut self, id: u32, ui: &mut Ui, theme: &Theme) {
        let Some((tag, props)) = self
            .slot(id)
            .map(|n| (n.tag.clone(), Props(n.props.clone())))
        else {
            return;
        };
        let enabled = props.bool("sensitive", true);
        // `:scroll-here` brings this node into view in whatever scroll area it
        // sits in. It fires on every frame the prop is set, so a caller sets it
        // for the moment of a jump and takes it off again — leaving it on would
        // pin the area there and take scrolling away from the reader.
        let scroll_here = props.bool("scroll-here", false);
        let before = ui.cursor().top();
        self.with_width(&props, ui, |tree, ui| {
            if enabled {
                tree.paint_tag(id, &tag, &props, ui, theme);
            } else {
                // Scoped rather than per-widget: a dimmed container dims its
                // whole subtree, which is what `:sensitive false` means
                // everywhere else in glimmer.
                ui.add_enabled_ui(false, |ui| tree.paint_tag(id, &tag, &props, ui, theme));
            }
        });
        if scroll_here {
            let rect = egui::Rect::from_min_max(
                egui::pos2(ui.max_rect().left(), before),
                egui::pos2(ui.max_rect().right(), ui.cursor().top()),
            );
            ui.scroll_to_rect(rect, Some(Align::Center));
        }
    }

    /// Constrain `add` to the node's `:width-request`, when it has one.
    ///
    /// Immediate mode has no natural width for a field: an entry asks for
    /// whatever is left, so an entry beside a button in an `:hbox` takes the
    /// row and wraps the button onto the next line. This is how a caller says
    /// otherwise.
    fn with_width(&mut self, props: &Props, ui: &mut Ui, add: impl FnOnce(&mut Self, &mut Ui)) {
        let requested = props.num("width-request", 0.0) as f32;
        if requested <= 0.0 {
            add(self, ui);
            return;
        }
        let width = requested.min(ui.available_width().max(1.0));
        // The height is the row's, not zero: a region allocated with no height
        // leaves the row measuring nothing at the moment the next widget is
        // placed, so a button beside a text field lands at the row's top edge
        // instead of beside it.
        let height = ui.available_height().max(0.0);
        ui.allocate_ui_with_layout(
            Vec2::new(width, height),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_min_width(width);
                ui.set_max_width(width);
                add(self, ui);
            },
        );
    }

    fn paint_tag(&mut self, id: u32, tag: &Tag, props: &Props, ui: &mut Ui, theme: &Theme) {
        match tag {
            // The root is the window itself: its children stack down the page.
            Tag::Window => self.paint_children(id, ui, theme),

            Tag::Box | Tag::Unknown => {
                let horizontal = props.str("orientation") == "horizontal";
                let spacing = props.num("spacing", theme.spacing.sm as f64) as f32;
                self.with_margin(props, ui, |tree, ui| {
                    let axis = if horizontal {
                        Vec2::new(spacing, ui.spacing().item_spacing.y)
                    } else {
                        Vec2::new(ui.spacing().item_spacing.x, spacing)
                    };
                    if horizontal {
                        // `:align :end` lays the row out from the right edge of
                        // the space it is given, which is how a trailing group
                        // — an action beside a message, a count beside a name —
                        // sits against the right of a row rather than trailing
                        // whatever came before it.
                        if props.str("align") == "end" {
                            // Nested in a row of its own: a right-to-left
                            // layout takes the height available to it, which
                            // in a column is everything below — every such row
                            // would be as tall as the rest of the screen, and
                            // the gaps would land between the rows above it.
                            ui.horizontal(|ui| {
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.spacing_mut().item_spacing = axis;
                                    tree.paint_children(id, ui, theme);
                                });
                            });
                        } else {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = axis;
                                tree.paint_children(id, ui, theme);
                            });
                        }
                    } else {
                        // `:align :center` puts a column's children on the
                        // middle of the width rather than against its left
                        // edge — what a picture on a screen of its own wants,
                        // and nothing a column of text ever does.
                        let cross = if props.str("align") == "center" {
                            Align::Center
                        } else {
                            Align::Min
                        };
                        ui.with_layout(Layout::top_down(cross), |ui| {
                            ui.spacing_mut().item_spacing = axis;
                            tree.paint_children(id, ui, theme);
                        });
                    }
                });
            }

            // A scrolling column with page padding, optionally centred at a
            // maximum width — the shell most Vidya apps put everything inside.
            Tag::Page => {
                let max_width = props.num("max-width", 0.0) as f32;
                let pad = theme.spacing.page;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::new()
                            .inner_margin(Margin::same(pad.clamp(0.0, 127.0) as i8))
                            .show(ui, |ui| {
                                let avail = ui.available_width();
                                let width = if max_width > 0.0 {
                                    max_width.min(avail)
                                } else {
                                    avail
                                };
                                let indent = ((avail - width) * 0.5).max(0.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(indent);
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(width, 0.0),
                                        Layout::top_down(Align::Min),
                                        |ui| {
                                            ui.set_min_width(width);
                                            ui.set_max_width(width);
                                            vidya_core::vstack(ui, theme, |ui| {
                                                self.paint_children(id, ui, theme);
                                            });
                                        },
                                    );
                                });
                            });
                    });
            }

            Tag::Scroll => {
                let area = match props.str("orientation") {
                    "horizontal" => egui::ScrollArea::horizontal(),
                    "both" => egui::ScrollArea::both(),
                    _ => egui::ScrollArea::vertical(),
                };
                // Without a bound a scroll area takes every point left in its
                // parent, so anything after it — a compose bar under a message
                // list — is pushed off the bottom. `:max-height` bounds it
                // outright; `:reserve` bounds it by what it must leave behind,
                // which is what a caller actually knows: the compose bar's
                // height, not the window's.
                let area = {
                    let reserve = props.num("reserve", 0.0) as f32;
                    let max_height = if reserve > 0.0 {
                        // Clamped against the clip rect as well as the layout's
                        // own idea of what is left: on Android the two differ
                        // once the soft keyboard takes the bottom of the
                        // screen, and it is the visible one that has to win or
                        // the row below the list is pushed off under the
                        // keyboard.
                        let visible = (ui.clip_rect().bottom() - ui.cursor().top()).max(0.0);
                        (ui.available_height().min(visible) - reserve).max(0.0)
                    } else {
                        props.num("max-height", 0.0) as f32
                    };
                    if max_height > 0.0 {
                        area.max_height(max_height)
                    } else {
                        area
                    }
                };
                // Keyed by the node rather than by where it sits: egui derives
                // a scroll area's id from its parent ui, so two areas that
                // occupy the same place in the tree at different times — the
                // message list and the picture that replaces the screen it is
                // on — would otherwise share one offset, and the list would
                // come back showing whatever the picture left behind.
                //
                // `:scroll-key` names an area that outlives its node instead.
                // A node id is only as durable as the node: a list unmounted
                // while another screen is up comes back as a new node, and a
                // position keyed by that is a position thrown away. A caller
                // that means "this same list again" says so with a name, and
                // the reader returns to the line they left.
                let key = {
                    let name = props.str("scroll-key");
                    if name.is_empty() {
                        Id::new(("vidya_scroll", id))
                    } else {
                        Id::new(("vidya_scroll_key", name))
                    }
                };
                let area = area.id_salt(key);
                // A chat wants the newest line, not the oldest — except on a
                // frame where something inside asked to be scrolled to. The
                // two are the same control pulling opposite ways, and sticking
                // wins every time it is asked, so a jump to an old message
                // would land nowhere.
                let sticks = props.bool("stick-to-bottom", false) && !self.wants_scroll_to(id);
                let area = area.stick_to_bottom(sticks);
                // `:scroll-to-bottom` is a number the caller bumps rather than
                // a flag it sets: a flag would have to be cleared afterwards,
                // and there is no frame in which the caller could do it. A
                // value it has not seen before means "now".
                let jump_key = key.with("jump");
                let jump = props.num("scroll-to-bottom", 0.0);
                let jumped = ui.ctx().data(|d| d.get_temp::<f64>(jump_key));
                let jump_now = jump > 0.0 && jumped != Some(jump);
                // The end is last frame's own maximum offset, kept for exactly
                // this. Not f32::MAX — egui subtracts the viewport from what it
                // is given, and MAX minus anything is still MAX, an offset the
                // content can never reach: the area painted nothing and stayed
                // that way. Not `scroll_to_rect` either, which a scroll area
                // that has been scrolled away from ignores here.
                let end_offset_key = key.with("end_offset");
                let area = if jump_now {
                    ui.ctx().data_mut(|d| d.insert_temp(jump_key, jump));
                    let end = ui
                        .ctx()
                        .data(|d| d.get_temp::<f32>(end_offset_key))
                        .unwrap_or(0.0);
                    area.vertical_scroll_offset(end)
                } else {
                    area
                };
                // Hold the content to the viewport's width, as `:page` does,
                // so a wrapping child wraps at the visible edge.
                let viewport_width = ui.available_width();
                let output = area.auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_max_width(viewport_width);
                    self.paint_children(id, ui, theme);
                    // The end asked for by scrolling to it, not by setting an
                    // offset of f32::MAX: egui subtracts the viewport from
                    // whatever it is given, and MAX minus anything is still
                    // MAX — an offset the content can never reach, which left
                    // the area painting nothing at all.

                });

                // Say when the view leaves the end and when it comes back, so
                // a caller can offer the way back. Reported on change only: the
                // position itself changes every frame of a scroll, and an event
                // a frame is not news.
                // Within a line of the end counts as the end, and content
                // shorter than the viewport is always at it.
                let max_offset = (output.content_size.y - output.inner_rect.height()).max(0.0);
                // What `:scroll-to-bottom` will aim at next time it is asked.
                ui.ctx()
                    .data_mut(|d| d.insert_temp(end_offset_key, max_offset));
                let at_end = output.state.offset.y >= max_offset - 24.0;
                // Reaching the end is reported at once; leaving it has to hold
                // for a few frames first. A burst of arriving messages grows
                // the content faster than the offset follows it, and reporting
                // that honestly would blink "scrolled away" whenever a channel
                // is busy.
                let end_key = key.with("at_end");
                let away_key = key.with("away_frames");
                let away_frames = ui.ctx().data(|d| d.get_temp::<u32>(away_key)).unwrap_or(0);
                let away_frames = if at_end { 0 } else { away_frames.saturating_add(1) };
                ui.ctx().data_mut(|d| d.insert_temp(away_key, away_frames));

                let settled = if at_end {
                    Some(true)
                } else if away_frames >= 3 {
                    Some(false)
                } else {
                    None
                };
                if let Some(at_end) = settled {
                    let was_at_end = ui.ctx().data(|d| d.get_temp::<bool>(end_key));
                    if was_at_end != Some(at_end) {
                        ui.ctx().data_mut(|d| d.insert_temp(end_key, at_end));
                        // Only after the first report: the opening one would
                        // arrive before the content has a height.
                        if was_at_end.is_some() {
                            self.emit(
                                id,
                                "change",
                                if at_end { "end" } else { "away" }.to_owned(),
                                if at_end { 1.0 } else { 0.0 },
                            );
                        }
                    }
                }
            }

            Tag::Card => {
                vidya_core::card(ui, theme, |ui| self.paint_children(id, ui, theme));
            }

            // A card with a heading — glimmer-tui's `:frame` label, in the
            // idiom this theme actually has for one.
            Tag::Frame => {
                let label = props.label();
                vidya_core::card(ui, theme, |ui| {
                    if !label.is_empty() {
                        vidya_core::title_2(ui, theme, label);
                    }
                    self.paint_children(id, ui, theme);
                });
            }

            Tag::Label => vidya_core::body(ui, theme, props.label()),

            // Body text that answers the pointer: the accent colour and the
            // hand cursor are the whole affordance, and the click is reported
            // like a button's so the caller decides what opening it means.
            Tag::Link => {
                let response = ui
                    .add(
                        egui::Label::new(
                            egui::RichText::new(props.label())
                                .size(theme.type_scale.body)
                                .color(theme.palette.accent),
                        )
                        .wrap()
                        .sense(egui::Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if response.clicked() {
                    self.emit(id, "click", props.label().to_owned(), 0.0);
                }
            }
            Tag::Title => vidya_core::title(ui, theme, props.label()),
            Tag::Title2 => vidya_core::title_2(ui, theme, props.label()),
            Tag::DimLabel => vidya_core::dim_label(ui, theme, props.label()),

            Tag::Button => {
                let kind = match props.str("kind") {
                    "primary" => 1,
                    "destructive" => 2,
                    _ => 0,
                };
                if crate::ui::button(ui, theme, props.label(), kind) {
                    self.emit(id, "click", String::new(), 0.0);
                }
            }

            Tag::CheckButton => {
                let was = props.bool("active", false);
                let (now, changed) = crate::ui::checkbox(ui, theme, was, props.label());
                if changed {
                    // The widget does not own the value: the new state is
                    // written back so a component that ignores `:on-toggled`
                    // still tracks the click, and the handler decides whether
                    // it survives the next render of `:active`.
                    self.set(id, "active", Value::Bool(now));
                    self.emit(id, "toggled", String::new(), if now { 1.0 } else { 0.0 });
                }
            }

            Tag::Entry => {
                let mut text = props.str("text").to_owned();
                let placeholder = props.str("placeholder").to_owned();
                let rows = props.num("rows", 4.0) as usize;
                let response = if props.bool("multiline", false) {
                    vidya_core::text_field_multiline(ui, theme, &mut text, rows.max(1))
                } else {
                    crate::ui::text_field(ui, theme, &mut text, &placeholder)
                };
                if text != props.str("text") {
                    self.set(id, "text", Value::Str(text.clone()));
                    self.emit(id, "change", text, 0.0);
                }
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.emit(id, "activate", String::new(), 0.0);
                }
                // A paste of something that is not text. egui turns Ctrl+V
                // into a `Paste` event carrying the clipboard's text, and a
                // clipboard holding a picture has none — so the keystroke
                // arrives as a key press with no paste behind it, and the
                // field would otherwise swallow it. Reported instead, for a
                // caller that has somewhere to put a picture; one that has not
                // ignores it and the keystroke stays as inert as it was.
                //
                // The clipboard is not read here: whether there is a picture
                // on it is answered by `vidya_clipboard_image_png`, and asking
                // twice would copy every pasted image for nothing.
                if response.has_focus() {
                    let paste_without_text = ui.input(|i| {
                        i.events.iter().any(|e| {
                            matches!(
                                e,
                                egui::Event::Key {
                                    key: egui::Key::V,
                                    pressed: true,
                                    modifiers,
                                    ..
                                } if modifiers.command
                            )
                        }) && !i
                            .events
                            .iter()
                            .any(|e| matches!(e, egui::Event::Paste(_)))
                    });
                    if paste_without_text {
                        self.emit(id, "paste-empty", String::new(), 0.0);
                    }
                }
            }

            Tag::Separator => crate::ui::separator(ui),
            Tag::Spacer => crate::ui::gap(ui, props.num("size", theme.spacing.md as f64) as f32),
            Tag::Status => crate::ui::status(ui, theme, props.label(), props.bool("live", false)),

            Tag::Progress => {
                let value = props.num("value", 0.0) as f32;
                let mut bar = egui::ProgressBar::new(value.clamp(0.0, 1.0));
                if !props.label().is_empty() {
                    bar = bar.text(props.label());
                }
                ui.add(bar);
            }

            // A picture from a file the caller has already fetched. Decoded
            // once and kept as a texture: the tree is walked every frame, and
            // decoding a PNG sixty times a second is not a thing to do.
            // Someone's face, or the next best thing. A chat wants one column
            // of them down the left, so this is a fixed square whatever the
            // picture's own proportions are, and there is always something to
            // draw: a name with no picture behind it becomes its initial on a
            // colour of its own, which keeps the column straight and still
            // tells one person from another at a glance.
            Tag::Avatar => {
                let size = props.num("size", 24.0) as f32;
                let label = props.label().to_owned();
                let path = props.str("src").to_owned();
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());

                let texture = if path.is_empty() {
                    None
                } else {
                    self.texture(ui, &path)
                };
                match texture {
                    // A corner radius of half the side is a circle.
                    Some(texture) => egui::Image::new(egui::load::SizedTexture::new(
                        texture.id(),
                        Vec2::splat(size),
                    ))
                    .corner_radius(size * 0.5)
                    .paint_at(ui, rect),
                    None => {
                        let initial = label
                            .trim_start_matches(['#', '&', '@', '+', '%', '~'])
                            .chars()
                            .next()
                            .map(|c| c.to_uppercase().to_string())
                            .unwrap_or_else(|| "?".to_owned());
                        ui.painter()
                            .circle_filled(rect.center(), size * 0.5, name_colour(&label, theme));
                        ui.painter().text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            initial,
                            FontId::proportional((size * 0.45).max(9.0)),
                            theme.palette.accent_fg,
                        );
                    }
                }
                if response.clicked() {
                    self.emit(id, "click", label, 0.0);
                }
            }

            // A reaction chip: the emoji drawn from the Twemoji pack rather
            // than set as text, so it is the colour picture people expect and
            // not a monochrome glyph — or, where the font has no glyph at all,
            // tofu. `:count` rides beside it once more than one person is on
            // it, and `:mine` is what marks the ones you put there yourself.
            Tag::Reaction => {
                let emoji = props.str("emoji").to_owned();
                let emoji = if emoji.is_empty() {
                    props.label().to_owned()
                } else {
                    emoji
                };
                let count = props.num("count", 0.0).max(0.0) as usize;
                let mine = props.bool("mine", false);
                // `:size` is the glyph's, and the pill is sized from it.
                let size = props.num("size", 0.0) as f32;
                let response = if size > 0.0 {
                    vidya_core::reaction_chip_sized(ui, theme, &emoji, count, mine, size)
                } else {
                    vidya_core::reaction_chip(ui, theme, &emoji, count, mine)
                };
                if response.clicked() {
                    self.emit(id, "click", emoji, count as f64);
                }
            }

            Tag::Image => {
                let path = props.str("src").to_owned();
                if path.is_empty() {
                    return;
                }
                let max_width = props.num("max-width", 0.0) as f32;
                let Some(texture) = self.texture(ui, &path) else {
                    // A file that will not decode is not worth a broken-image
                    // glyph; the message text beside it already says what it
                    // was meant to be.
                    return;
                };
                let size = texture.size_vec2();

                // `:fit` gives the picture every point of the space it has
                // been handed and centres it in it — a picture on a screen of
                // its own, rather than one in a line of chat. It is the one
                // case that scales *up*: a picture opened to be looked at is
                // meant to fill the window, and how big the window is this
                // frame is something only this side knows. Everywhere else the
                // caller's `:max-height` bounds it and nothing is enlarged
                // past its own pixels.
                if props.bool("fit", false) {
                    let space = ui.available_size();
                    if space.x <= 0.0 || space.y <= 0.0 || size.x <= 0.0 || size.y <= 0.0 {
                        return;
                    }
                    let scale = (space.x / size.x).min(space.y / size.y);
                    let (rect, response) =
                        ui.allocate_exact_size(space, egui::Sense::click());
                    let painted =
                        egui::Rect::from_center_size(rect.center(), size * scale);
                    egui::Image::new(egui::load::SizedTexture::new(texture.id(), size * scale))
                        .paint_at(ui, painted);
                    if response.clicked() {
                        self.emit(id, "click", String::new(), 0.0);
                    }
                    return;
                }

                let max_height = props.num("max-height", 240.0) as f32;
                let avail = if max_width > 0.0 {
                    max_width.min(ui.available_width())
                } else {
                    ui.available_width()
                };
                let scale = (avail / size.x).min(max_height / size.y).min(1.0);
                // Clickable whether or not the caller listens: the tree does
                // not know which nodes have handlers, and an unheard event
                // costs a queue slot.
                let response = ui
                    .add(
                        egui::Image::new(egui::load::SizedTexture::new(texture.id(), size * scale))
                            .corner_radius(theme.spacing.radius_sm)
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if response.clicked() {
                    self.emit(id, "click", String::new(), 0.0);
                }
            }

            Tag::Spinner => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    if !props.label().is_empty() {
                        vidya_core::body(ui, theme, props.label());
                    }
                });
            }
        }
    }

    /// Wrap `add` in the node's `:margin`, when it has one.
    fn with_margin(&mut self, props: &Props, ui: &mut Ui, add: impl FnOnce(&mut Self, &mut Ui)) {
        // `:margin` sets all four sides; `:margin-top` and its siblings say
        // otherwise for one of them. A row that sits at the bottom of a screen
        // wants its space above it, not under it, and that is not a thing a
        // single number can express.
        let side = |key: &str| {
            props.num(key, props.num("margin", 0.0)).clamp(0.0, 127.0) as i8
        };
        let margin = Margin {
            left: side("margin-left"),
            right: side("margin-right"),
            top: side("margin-top"),
            bottom: side("margin-bottom"),
        };
        if margin == Margin::ZERO {
            add(self, ui);
            return;
        }
        egui::Frame::new()
            .inner_margin(margin)
            .show(ui, |ui| add(self, ui));
    }
}

/// Typed reads over a node's prop map, with the defaults each widget wants.
struct Props(HashMap<String, Value>);

impl Props {
    fn str(&self, key: &str) -> &str {
        match self.0.get(key) {
            Some(Value::Str(s)) => s,
            _ => "",
        }
    }

    fn num(&self, key: &str, default: f64) -> f64 {
        match self.0.get(key) {
            Some(Value::Num(n)) => *n,
            Some(Value::Bool(b)) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            _ => default,
        }
    }

    fn bool(&self, key: &str, default: bool) -> bool {
        match self.0.get(key) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Num(n)) => *n != 0.0,
            _ => default,
        }
    }

    /// `:label` is the family's name for a widget's text; `:text` is what a
    /// label is also allowed to use (and what an entry always uses).
    fn label(&self) -> &str {
        let label = self.str("label");
        if label.is_empty() {
            self.str("text")
        } else {
            label
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kids(tree: &Tree, id: u32) -> Vec<u32> {
        tree.slot(id)
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }

    #[test]
    fn root_exists_and_is_a_window() {
        let tree = Tree::default();
        assert!(tree.exists(tree.root()));
        assert_eq!(tree.slot(tree.root()).unwrap().tag, Tag::Window);
    }

    #[test]
    fn append_parents_once_even_when_reparenting() {
        let mut tree = Tree::default();
        let a = tree.new_node("vbox");
        let b = tree.new_node("hbox");
        let leaf = tree.new_node("label");
        tree.append(tree.root(), a);
        tree.append(tree.root(), b);

        tree.append(a, leaf);
        tree.append(b, leaf);
        assert_eq!(kids(&tree, a), vec![]);
        assert_eq!(kids(&tree, b), vec![leaf]);
    }

    #[test]
    fn a_cycle_is_refused() {
        let mut tree = Tree::default();
        let outer = tree.new_node("vbox");
        let inner = tree.new_node("vbox");
        tree.append(tree.root(), outer);
        tree.append(outer, inner);
        assert!(!tree.append(inner, outer));
        assert_eq!(kids(&tree, inner), vec![]);
    }

    #[test]
    fn remove_frees_the_whole_subtree_and_reuses_slots() {
        let mut tree = Tree::default();
        let parent = tree.new_node("vbox");
        let child = tree.new_node("label");
        tree.append(tree.root(), parent);
        tree.append(parent, child);

        tree.remove(tree.root(), parent);
        assert!(!tree.exists(parent));
        assert!(!tree.exists(child));
        assert_eq!(tree.new_node("label"), child);
    }

    #[test]
    fn remove_ignores_a_child_of_someone_else() {
        let mut tree = Tree::default();
        let a = tree.new_node("vbox");
        let b = tree.new_node("vbox");
        let leaf = tree.new_node("label");
        tree.append(tree.root(), a);
        tree.append(tree.root(), b);
        tree.append(a, leaf);

        tree.remove(b, leaf);
        assert!(tree.exists(leaf));
        assert_eq!(kids(&tree, a), vec![leaf]);
    }

    #[test]
    fn insert_after_reorders_in_both_directions() {
        let mut tree = Tree::default();
        let parent = tree.new_node("vbox");
        tree.append(tree.root(), parent);
        let a = tree.new_node("label");
        let b = tree.new_node("label");
        let c = tree.new_node("label");
        for id in [a, b, c] {
            tree.append(parent, id);
        }

        // Move a forward, past two siblings.
        assert!(tree.insert_after(parent, a, c));
        assert_eq!(kids(&tree, parent), vec![b, c, a]);
        // And back to the front.
        assert!(tree.insert_after(parent, a, 0));
        assert_eq!(kids(&tree, parent), vec![a, b, c]);
        // A no-op move keeps the order it already had.
        assert!(tree.insert_after(parent, b, a));
        assert_eq!(kids(&tree, parent), vec![a, b, c]);
    }

    #[test]
    fn replace_swaps_in_place_and_drops_the_old_node() {
        let mut tree = Tree::default();
        let parent = tree.new_node("vbox");
        tree.append(tree.root(), parent);
        let a = tree.new_node("label");
        let b = tree.new_node("label");
        let c = tree.new_node("button");
        tree.append(parent, a);
        tree.append(parent, b);

        assert!(tree.replace(parent, a, c));
        assert_eq!(kids(&tree, parent), vec![c, b]);
        assert!(!tree.exists(a));
    }

    #[test]
    fn props_round_trip_and_clear() {
        let mut tree = Tree::default();
        let id = tree.new_node("button");
        tree.set(id, "label", Value::Str("Save".into()));
        tree.set(id, "value", Value::Num(0.5));
        tree.set(id, "active", Value::Bool(true));
        assert_eq!(tree.get(id, "label"), Some(&Value::Str("Save".into())));
        assert_eq!(tree.get(id, "value"), Some(&Value::Num(0.5)));
        assert_eq!(tree.get(id, "active"), Some(&Value::Bool(true)));

        tree.clear_props(id);
        assert_eq!(tree.get(id, "label"), None);
    }

    #[test]
    fn events_drain_in_order_and_skip_removed_nodes() {
        let mut tree = Tree::default();
        let a = tree.new_node("button");
        let b = tree.new_node("button");
        tree.append(tree.root(), a);
        tree.append(tree.root(), b);
        tree.emit(a, "click", String::new(), 0.0);
        tree.emit(b, "click", String::new(), 0.0);

        // Dropping `a` must drop the event still queued against it, or it would
        // be routed to a handler the caller has already forgotten.
        tree.remove(tree.root(), a);
        assert!(tree.poll());
        assert_eq!(tree.current().unwrap().node, b);
        assert!(!tree.poll());
        assert!(tree.current().is_none());
    }

    #[test]
    fn unknown_tags_are_kept_as_boxes() {
        let mut tree = Tree::default();
        let id = tree.new_node("carousel");
        assert!(tree.exists(id));
        assert_eq!(tree.slot(id).unwrap().tag, Tag::Unknown);
    }
}
