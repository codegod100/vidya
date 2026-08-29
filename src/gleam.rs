//! Declarative UI IR for designing Vidya interfaces from [Gleam](https://gleam.run).
//!
//! Gleam owns the model and `view` (see `gleam/vidya`). Each frame the view is
//! encoded as JSON [`Node`] / [`App`] values; Rust walks the tree with the
//! existing Vidya helpers and returns [`Event`]s (`click` / `check` / `input`)
//! that Gleam maps back to messages.
//!
//! Interactive widgets carry a stable string `id`. Values (checkbox checked,
//! text field contents) are owned by Gleam and passed in the tree each frame.

use egui::{Context, Ui};
use serde::{Deserialize, Serialize};

use crate::{
    body, button, card, checkbox, compact_card, destructive_button, dim_label, emoji_icon, grid_cols,
    hflow, icon as paint_named_icon, icon_button, inset_row, lead_trail, pack, page_body_cols,
    primary_button, reaction_chip, status_dot, text_field_multiline, text_field_singleline, title,
    title_2, top_header, two_col, vstack, ColSpec, Icon, Mode, Theme,
};

/// Shell mode for [`App`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    pub fn theme(self) -> Theme {
        match self {
            ThemeMode::Dark => Theme::dark(),
            ThemeMode::Light => Theme::light(),
        }
    }

    pub fn mode(self) -> Mode {
        match self {
            ThemeMode::Dark => Mode::Dark,
            ThemeMode::Light => Mode::Light,
        }
    }
}

/// Column width hint — mirrors [`ColSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Col {
    Flex,
    Fixed { px: f32 },
    #[serde(rename = "metric_bps")]
    MetricBps,
    #[serde(rename = "metric_rate")]
    MetricRate,
}

impl Col {
    pub fn to_spec(&self) -> ColSpec {
        match self {
            Col::Flex => ColSpec::Flex,
            Col::Fixed { px } => ColSpec::Fixed(*px),
            Col::MetricBps => ColSpec::MetricBps,
            Col::MetricRate => ColSpec::MetricRate,
        }
    }
}

/// Named icon — mirrors [`Icon`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IconName {
    #[serde(rename = "thumbs_up")]
    ThumbsUp,
    #[serde(rename = "thumbs_down")]
    ThumbsDown,
    Heart,
    Laugh,
    Surprised,
    Frown,
    Plus,
    Copy,
}

impl IconName {
    pub fn to_icon(self) -> Icon {
        match self {
            IconName::ThumbsUp => Icon::ThumbsUp,
            IconName::ThumbsDown => Icon::ThumbsDown,
            IconName::Heart => Icon::Heart,
            IconName::Laugh => Icon::Laugh,
            IconName::Surprised => Icon::Surprised,
            IconName::Frown => Icon::Frown,
            IconName::Plus => Icon::Plus,
            IconName::Copy => Icon::Copy,
        }
    }
}

/// Interaction emitted while rendering a tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    Click { id: String },
    Check { id: String, checked: bool },
    Input { id: String, value: String },
}

/// Top-level application view produced by Gleam.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct App {
    #[serde(default)]
    pub theme: ThemeMode,
    /// Optional contents for [`top_header`] (Android chrome-aware).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<Node>,
    pub page: Page,
}

/// Grid-enforced central page (Vidya's supported app root).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    #[serde(default)]
    pub cols: Vec<Col>,
    pub sections: Vec<Node>,
}

/// One declarative UI node. Layout containers nest children; controls carry `id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Node {
    Title {
        text: String,
    },
    #[serde(rename = "title_2")]
    Title2 {
        text: String,
    },
    Body {
        text: String,
    },
    Dim {
        text: String,
    },
    #[serde(rename = "primary_button")]
    PrimaryButton {
        id: String,
        label: String,
    },
    Button {
        id: String,
        label: String,
    },
    #[serde(rename = "destructive_button")]
    DestructiveButton {
        id: String,
        label: String,
    },
    #[serde(rename = "icon_button")]
    IconButton {
        id: String,
        icon: IconName,
        #[serde(default)]
        tip: String,
    },
    Checkbox {
        id: String,
        label: String,
        checked: bool,
    },
    #[serde(rename = "text_field")]
    TextField {
        id: String,
        value: String,
        #[serde(default)]
        multiline: bool,
        #[serde(default = "default_rows")]
        rows: usize,
    },
    #[serde(rename = "status_dot")]
    StatusDot {
        live: bool,
    },
    Icon {
        icon: IconName,
        #[serde(default = "default_icon_size")]
        size: f32,
    },
    Emoji {
        emoji: String,
        #[serde(default = "default_icon_size")]
        size: f32,
    },
    #[serde(rename = "reaction_chip")]
    ReactionChip {
        id: String,
        emoji: String,
        #[serde(default)]
        count: usize,
        #[serde(default)]
        mine: bool,
    },
    #[serde(rename = "vstack")]
    VStack {
        children: Vec<Node>,
    },
    #[serde(rename = "hflow")]
    HFlow {
        children: Vec<Node>,
    },
    Pack {
        children: Vec<Node>,
    },
    Card {
        children: Vec<Node>,
    },
    #[serde(rename = "compact_card")]
    CompactCard {
        width: f32,
        children: Vec<Node>,
    },
    #[serde(rename = "inset_row")]
    InsetRow {
        children: Vec<Node>,
    },
    #[serde(rename = "lead_trail")]
    LeadTrail {
        lead: Box<Node>,
        trail: Vec<Node>,
    },
    #[serde(rename = "two_col")]
    TwoCol {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_col: Option<f32>,
        left: Box<Node>,
        right: Box<Node>,
    },
    Grid {
        id: String,
        cols: Vec<Col>,
        rows: Vec<Row>,
    },
    /// Nested free content (escape hatch inside a cell).
    Group {
        children: Vec<Node>,
    },
}

fn default_rows() -> usize {
    3
}

fn default_icon_size() -> f32 {
    22.0
}

/// One grid row: either free cells or typed metric/text cells.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Row {
    Cells { cells: Vec<Node> },
    Values { cells: Vec<Cell> },
}

/// Typed grid cell matching [`crate::RowDsl`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Cell {
    Heading { text: String },
    Text { text: String },
    Dim { text: String },
    Warn { text: String },
    Metric { text: String },
    #[serde(rename = "metric_dim")]
    MetricDim { text: String },
    #[serde(rename = "metric_bps")]
    MetricBps { bps: f64 },
    #[serde(rename = "metric_rate")]
    MetricRate { rate: f64 },
    /// Free-form nested widgets in this cell.
    Node { node: Node },
}

/// Apply theme + render a Gleam [`App`] into the egui context.
///
/// Returns interaction events for Gleam's `update`.
pub fn render_app(ctx: &Context, app: &App) -> Vec<Event> {
    let theme = app.theme.theme();
    crate::apply(ctx, &theme);
    let mut events = Vec::new();

    if let Some(header) = &app.header {
        top_header(ctx, &theme, |ui| {
            render_node(ui, &theme, header, &mut events);
        });
    } else {
        crate::reserve_system_chrome(ctx, &theme);
    }

    let cols: Vec<ColSpec> = if app.page.cols.is_empty() {
        vec![ColSpec::Flex]
    } else {
        app.page.cols.iter().map(Col::to_spec).collect()
    };

    egui::CentralPanel::default()
        .frame(theme.page_frame())
        .show(ctx, |ui| {
            page_body_cols(ui, &theme, app.page.id.as_str(), &cols, |g| {
                for section in &app.page.sections {
                    g.section(|ui| {
                        render_node(ui, &theme, section, &mut events);
                    });
                }
            });
        });

    events
}

/// Render a node tree into an existing `Ui` (nested / testing).
pub fn render_node(ui: &mut Ui, theme: &Theme, node: &Node, events: &mut Vec<Event>) {
    match node {
        Node::Title { text } => title(ui, theme, text),
        Node::Title2 { text } => title_2(ui, theme, text),
        Node::Body { text } => body(ui, theme, text),
        Node::Dim { text } => dim_label(ui, theme, text),
        Node::PrimaryButton { id, label } => {
            if primary_button(ui, theme, label).clicked() {
                events.push(Event::Click { id: id.clone() });
            }
        }
        Node::Button { id, label } => {
            if button(ui, theme, label).clicked() {
                events.push(Event::Click { id: id.clone() });
            }
        }
        Node::DestructiveButton { id, label } => {
            if destructive_button(ui, theme, label).clicked() {
                events.push(Event::Click { id: id.clone() });
            }
        }
        Node::IconButton { id, icon, tip } => {
            if icon_button(ui, theme, icon.to_icon(), tip).clicked() {
                events.push(Event::Click { id: id.clone() });
            }
        }
        Node::Checkbox { id, label, checked } => {
            let mut value = *checked;
            let response = checkbox(ui, theme, &mut value, label);
            if response.changed() {
                events.push(Event::Check {
                    id: id.clone(),
                    checked: value,
                });
            }
        }
        Node::TextField {
            id,
            value,
            multiline,
            rows,
        } => {
            let mut buf = value.clone();
            let response = if *multiline {
                text_field_multiline(ui, theme, &mut buf, (*rows).max(1))
            } else {
                text_field_singleline(ui, theme, &mut buf)
            };
            if response.changed() {
                events.push(Event::Input {
                    id: id.clone(),
                    value: buf,
                });
            }
        }
        Node::StatusDot { live } => {
            status_dot(ui, theme, *live);
        }
        Node::Icon { icon, size } => {
            paint_named_icon(ui, theme, icon.to_icon(), *size);
        }
        Node::Emoji { emoji, size } => {
            emoji_icon(ui, theme, emoji, *size);
        }
        Node::ReactionChip {
            id,
            emoji,
            count,
            mine,
        } => {
            if reaction_chip(ui, theme, emoji, *count, *mine).clicked() {
                events.push(Event::Click { id: id.clone() });
            }
        }
        Node::VStack { children } => {
            vstack(ui, theme, |ui| {
                for child in children {
                    render_node(ui, theme, child, events);
                }
            });
        }
        Node::HFlow { children } => {
            hflow(ui, theme, |ui| {
                for child in children {
                    render_node(ui, theme, child, events);
                }
            });
        }
        Node::Pack { children } => {
            pack(ui, theme, |ui| {
                for child in children {
                    render_node(ui, theme, child, events);
                }
            });
        }
        Node::Card { children } => {
            card(ui, theme, |ui| {
                for child in children {
                    render_node(ui, theme, child, events);
                }
            });
        }
        Node::CompactCard { width, children } => {
            compact_card(ui, theme, *width, |ui| {
                for child in children {
                    render_node(ui, theme, child, events);
                }
            });
        }
        Node::InsetRow { children } => {
            inset_row(ui, theme, |ui| {
                for child in children {
                    render_node(ui, theme, child, events);
                }
            });
        }
        Node::LeadTrail { lead, trail } => {
            let mut lead_events = Vec::new();
            let mut trail_events = Vec::new();
            lead_trail(
                ui,
                |ui| render_node(ui, theme, lead, &mut lead_events),
                |ui| {
                    for child in trail {
                        render_node(ui, theme, child, &mut trail_events);
                    }
                },
            );
            events.extend(lead_events);
            events.extend(trail_events);
        }
        Node::TwoCol {
            min_col,
            left,
            right,
        } => {
            let min = min_col.unwrap_or_else(|| crate::default_min_col(theme));
            let mut left_events = Vec::new();
            let mut right_events = Vec::new();
            two_col(
                ui,
                theme,
                min,
                |ui| render_node(ui, theme, left, &mut left_events),
                |ui| render_node(ui, theme, right, &mut right_events),
            );
            events.extend(left_events);
            events.extend(right_events);
        }
        Node::Grid { id, cols, rows } => {
            let specs: Vec<ColSpec> = cols.iter().map(Col::to_spec).collect();
            grid_cols(ui, theme, id.as_str(), &specs, |g| {
                for row in rows {
                    match row {
                        Row::Cells { cells } => {
                            g.row(|r| {
                                for cell in cells {
                                    r.cell(|ui| render_node(ui, theme, cell, events));
                                }
                            });
                        }
                        Row::Values { cells } => {
                            g.row(|r| {
                                for cell in cells {
                                    render_cell(r, theme, cell, events);
                                }
                            });
                        }
                    }
                }
            });
        }
        Node::Group { children } => {
            for child in children {
                render_node(ui, theme, child, events);
            }
        }
    }
}

fn render_cell(
    row: &mut crate::RowDsl<'_, '_>,
    theme: &Theme,
    cell: &Cell,
    events: &mut Vec<Event>,
) {
    match cell {
        Cell::Heading { text } => row.heading(text),
        Cell::Text { text } => row.text(text),
        Cell::Dim { text } => row.dim(text),
        Cell::Warn { text } => row.warn(text),
        Cell::Metric { text } => row.metric(text),
        Cell::MetricDim { text } => row.metric_dim(text),
        Cell::MetricBps { bps } => row.metric_bps(*bps),
        Cell::MetricRate { rate } => row.metric_rate(*rate),
        Cell::Node { node } => {
            row.cell(|ui| render_node(ui, theme, node, events));
        }
    }
}

/// Parse an [`App`] from Gleam-produced JSON.
pub fn app_from_json(json: &str) -> Result<App, serde_json::Error> {
    serde_json::from_str(json)
}

/// Parse a [`Node`] from Gleam-produced JSON.
pub fn node_from_json(json: &str) -> Result<Node, serde_json::Error> {
    serde_json::from_str(json)
}

/// Encode events for Gleam's decoder.
pub fn events_to_json(events: &[Event]) -> Result<String, serde_json::Error> {
    serde_json::to_string(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_showcase_app() {
        let app = App {
            theme: ThemeMode::Dark,
            header: Some(Node::HFlow {
                children: vec![
                    Node::Title2 {
                        text: "Vidya".into(),
                    },
                    Node::PrimaryButton {
                        id: "toast".into(),
                        label: "Ping".into(),
                    },
                ],
            }),
            page: Page {
                id: "main".into(),
                cols: vec![Col::Flex],
                sections: vec![
                    Node::Card {
                        children: vec![
                            Node::Title {
                                text: "Hello from Gleam".into(),
                            },
                            Node::Body {
                                text: "Declarative mapping over Vidya widgets.".into(),
                            },
                            Node::Checkbox {
                                id: "sync".into(),
                                label: "Sync".into(),
                                checked: true,
                            },
                            Node::TextField {
                                id: "name".into(),
                                value: "nandi".into(),
                                multiline: false,
                                rows: 1,
                            },
                            Node::HFlow {
                                children: vec![
                                    Node::PrimaryButton {
                                        id: "save".into(),
                                        label: "Save".into(),
                                    },
                                    Node::DestructiveButton {
                                        id: "reset".into(),
                                        label: "Reset".into(),
                                    },
                                    Node::StatusDot { live: true },
                                ],
                            },
                        ],
                    },
                    Node::Grid {
                        id: "metrics".into(),
                        cols: vec![Col::Flex, Col::MetricBps, Col::MetricRate],
                        rows: vec![
                            Row::Values {
                                cells: vec![
                                    Cell::Heading {
                                        text: "Name".into(),
                                    },
                                    Cell::Heading {
                                        text: "Write".into(),
                                    },
                                    Cell::Heading {
                                        text: "Freq".into(),
                                    },
                                ],
                            },
                            Row::Values {
                                cells: vec![
                                    Cell::Text {
                                        text: "chrome".into(),
                                    },
                                    Cell::MetricBps { bps: 1_250_000.0 },
                                    Cell::MetricRate { rate: 42.0 },
                                ],
                            },
                        ],
                    },
                ],
            },
        };

        let json = serde_json::to_string_pretty(&app).unwrap();
        let parsed: App = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, app);

        // Events encode with snake_case tags Gleam expects.
        let ev = vec![
            Event::Click { id: "save".into() },
            Event::Check {
                id: "sync".into(),
                checked: false,
            },
            Event::Input {
                id: "name".into(),
                value: "x".into(),
            },
        ];
        let ev_json = events_to_json(&ev).unwrap();
        assert!(ev_json.contains(r#""kind":"click""#));
        assert!(ev_json.contains(r#""kind":"check""#));
        assert!(ev_json.contains(r#""kind":"input""#));
    }

    #[test]
    fn node_kind_tags_are_snake_case() {
        let n = Node::PrimaryButton {
            id: "a".into(),
            label: "A".into(),
        };
        let v = serde_json::to_value(&n).unwrap();
        assert_eq!(v["kind"], "primary_button");
    }

    #[test]
    fn parses_gleam_example_fixture() {
        let json = include_str!("../gleam/example/demo_app.json");
        let app = app_from_json(json).expect("gleam example JSON");
        assert_eq!(app.theme, ThemeMode::Dark);
        assert_eq!(app.page.id, "gleam-main");
        assert_eq!(app.page.sections.len(), 3);
        assert!(app.header.is_some());
    }
}
