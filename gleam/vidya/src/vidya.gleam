//// Declarative builders for every Vidya UI component.
////
//// Build an [`App`] in Gleam, [`encode_app`] it to JSON, and render with
//// Rust `vidya::gleam::render_app`. Interactive widgets take a stable string
//// `id`; events come back as [`Event`] values (`click` / `check` / `input`).

import gleam/json
import gleam/dynamic/decode
import gleam/list
import gleam/option.{type Option, None, Some}

// ── Theme / page shell ──────────────────────────────────────────────────────

pub type ThemeMode {
  Dark
  Light
}

pub type Col {
  Flex
  Fixed(px: Float)
  MetricBps
  MetricRate
}

pub type Icon {
  ThumbsUp
  ThumbsDown
  Heart
  Laugh
  Surprised
  Frown
  Plus
  Copy
}

pub type App {
  App(theme: ThemeMode, header: Option(Node), page: Page)
}

pub type Page {
  Page(id: String, cols: List(Col), sections: List(Node))
}

/// One declarative UI node. Layout containers nest children; controls carry `id`.
pub type Node {
  Title(text: String)
  Title2(text: String)
  Body(text: String)
  Dim(text: String)
  PrimaryButton(id: String, label: String)
  Button(id: String, label: String)
  DestructiveButton(id: String, label: String)
  IconButton(id: String, icon: Icon, tip: String)
  Checkbox(id: String, label: String, checked: Bool)
  TextField(id: String, value: String, multiline: Bool, rows: Int)
  StatusDot(live: Bool)
  NamedIcon(icon: Icon, size: Float)
  Emoji(emoji: String, size: Float)
  ReactionChip(id: String, emoji: String, count: Int, mine: Bool)
  VStack(children: List(Node))
  HFlow(children: List(Node))
  Pack(children: List(Node))
  Card(children: List(Node))
  CompactCard(width: Float, children: List(Node))
  InsetRow(children: List(Node))
  LeadTrail(lead: Node, trail: List(Node))
  TwoCol(min_col: Option(Float), left: Node, right: Node)
  Grid(id: String, cols: List(Col), rows: List(Row))
  Group(children: List(Node))
}

pub type Row {
  Cells(cells: List(Node))
  Values(cells: List(Cell))
}

pub type Cell {
  Heading(text: String)
  Text(text: String)
  DimCell(text: String)
  Warn(text: String)
  Metric(text: String)
  MetricDim(text: String)
  MetricBpsCell(bps: Float)
  MetricRateCell(rate: Float)
  Nested(node: Node)
}

pub type Event {
  Click(id: String)
  Check(id: String, checked: Bool)
  Input(id: String, value: String)
}

// ── Convenient constructors ─────────────────────────────────────────────────

pub fn app(page: Page) -> App {
  App(theme: Dark, header: None, page: page)
}

pub fn app_with_header(header: Node, page: Page) -> App {
  App(theme: Dark, header: Some(header), page: page)
}

pub fn page(id: String, sections: List(Node)) -> Page {
  Page(id: id, cols: [Flex], sections: sections)
}

pub fn page_cols(id: String, cols: List(Col), sections: List(Node)) -> Page {
  Page(id: id, cols: cols, sections: sections)
}

pub fn title(text: String) -> Node {
  Title(text)
}

pub fn title_2(text: String) -> Node {
  Title2(text)
}

pub fn body(text: String) -> Node {
  Body(text)
}

pub fn dim(text: String) -> Node {
  Dim(text)
}

pub fn primary_button(id: String, label: String) -> Node {
  PrimaryButton(id, label)
}

pub fn button(id: String, label: String) -> Node {
  Button(id, label)
}

pub fn destructive_button(id: String, label: String) -> Node {
  DestructiveButton(id, label)
}

pub fn icon_button(id: String, icon: Icon, tip: String) -> Node {
  IconButton(id, icon, tip)
}

pub fn checkbox(id: String, label: String, checked: Bool) -> Node {
  Checkbox(id, label, checked)
}

pub fn text_field(id: String, value: String) -> Node {
  TextField(id, value, False, 1)
}

pub fn text_field_multiline(id: String, value: String, rows: Int) -> Node {
  TextField(id, value, True, rows)
}

pub fn status_dot(live: Bool) -> Node {
  StatusDot(live)
}

pub fn icon(icon: Icon) -> Node {
  NamedIcon(icon, 22.0)
}

pub fn icon_sized(icon: Icon, size: Float) -> Node {
  NamedIcon(icon, size)
}

pub fn emoji(emoji: String) -> Node {
  Emoji(emoji, 22.0)
}

pub fn emoji_sized(emoji: String, size: Float) -> Node {
  Emoji(emoji, size)
}

pub fn reaction_chip(id: String, emoji: String, count: Int, mine: Bool) -> Node {
  ReactionChip(id, emoji, count, mine)
}

pub fn vstack(children: List(Node)) -> Node {
  VStack(children)
}

pub fn hflow(children: List(Node)) -> Node {
  HFlow(children)
}

pub fn pack(children: List(Node)) -> Node {
  Pack(children)
}

pub fn card(children: List(Node)) -> Node {
  Card(children)
}

pub fn compact_card(width: Float, children: List(Node)) -> Node {
  CompactCard(width, children)
}

pub fn inset_row(children: List(Node)) -> Node {
  InsetRow(children)
}

pub fn lead_trail(lead: Node, trail: List(Node)) -> Node {
  LeadTrail(lead, trail)
}

pub fn two_col(left: Node, right: Node) -> Node {
  TwoCol(None, left, right)
}

pub fn two_col_min(min_col: Float, left: Node, right: Node) -> Node {
  TwoCol(Some(min_col), left, right)
}

pub fn grid(id: String, cols: List(Col), rows: List(Row)) -> Node {
  Grid(id, cols, rows)
}

pub fn group(children: List(Node)) -> Node {
  Group(children)
}

pub fn row_cells(cells: List(Node)) -> Row {
  Cells(cells)
}

pub fn row_values(cells: List(Cell)) -> Row {
  Values(cells)
}

pub fn heading(text: String) -> Cell {
  Heading(text)
}

pub fn cell_text(text: String) -> Cell {
  Text(text)
}

pub fn cell_dim(text: String) -> Cell {
  DimCell(text)
}

pub fn warn(text: String) -> Cell {
  Warn(text)
}

pub fn metric(text: String) -> Cell {
  Metric(text)
}

pub fn metric_dim(text: String) -> Cell {
  MetricDim(text)
}

pub fn metric_bps(bps: Float) -> Cell {
  MetricBpsCell(bps)
}

pub fn metric_rate(rate: Float) -> Cell {
  MetricRateCell(rate)
}

pub fn nested(node: Node) -> Cell {
  Nested(node)
}

// ── JSON encode (Rust `vidya::gleam` IR) ─────────────────────────────────────

pub fn encode_app(app: App) -> String {
  json.to_string(app_to_json(app))
}

pub fn encode_node(node: Node) -> String {
  json.to_string(node_to_json(node))
}

pub fn app_to_json(app: App) -> json.Json {
  let App(theme:, header:, page:) = app
  let base = [
    #("theme", theme_to_json(theme)),
    #("page", page_to_json(page)),
  ]
  case header {
    Some(h) ->
      json.object(list.append(base, [#("header", node_to_json(h))]))
    None -> json.object(base)
  }
}

fn page_to_json(page: Page) -> json.Json {
  let Page(id:, cols:, sections:) = page
  json.object([
    #("id", json.string(id)),
    #("cols", json.preprocessed_array(list.map(cols, col_to_json))),
    #("sections", json.preprocessed_array(list.map(sections, node_to_json))),
  ])
}

pub fn node_to_json(node: Node) -> json.Json {
  case node {
    Title(text) -> tagged("title", [#("text", json.string(text))])
    Title2(text) -> tagged("title_2", [#("text", json.string(text))])
    Body(text) -> tagged("body", [#("text", json.string(text))])
    Dim(text) -> tagged("dim", [#("text", json.string(text))])
    PrimaryButton(id, label) ->
      tagged("primary_button", [
        #("id", json.string(id)),
        #("label", json.string(label)),
      ])
    Button(id, label) ->
      tagged("button", [
        #("id", json.string(id)),
        #("label", json.string(label)),
      ])
    DestructiveButton(id, label) ->
      tagged("destructive_button", [
        #("id", json.string(id)),
        #("label", json.string(label)),
      ])
    IconButton(id, icon, tip) ->
      tagged("icon_button", [
        #("id", json.string(id)),
        #("icon", icon_to_json(icon)),
        #("tip", json.string(tip)),
      ])
    Checkbox(id, label, checked) ->
      tagged("checkbox", [
        #("id", json.string(id)),
        #("label", json.string(label)),
        #("checked", json.bool(checked)),
      ])
    TextField(id, value, multiline, rows) ->
      tagged("text_field", [
        #("id", json.string(id)),
        #("value", json.string(value)),
        #("multiline", json.bool(multiline)),
        #("rows", json.int(rows)),
      ])
    StatusDot(live) -> tagged("status_dot", [#("live", json.bool(live))])
    NamedIcon(icon, size) ->
      tagged("icon", [
        #("icon", icon_to_json(icon)),
        #("size", json.float(size)),
      ])
    Emoji(emoji, size) ->
      tagged("emoji", [
        #("emoji", json.string(emoji)),
        #("size", json.float(size)),
      ])
    ReactionChip(id, emoji, count, mine) ->
      tagged("reaction_chip", [
        #("id", json.string(id)),
        #("emoji", json.string(emoji)),
        #("count", json.int(count)),
        #("mine", json.bool(mine)),
      ])
    VStack(children) ->
      tagged("vstack", [#("children", json.preprocessed_array(list.map(children, node_to_json)))])
    HFlow(children) ->
      tagged("hflow", [#("children", json.preprocessed_array(list.map(children, node_to_json)))])
    Pack(children) ->
      tagged("pack", [#("children", json.preprocessed_array(list.map(children, node_to_json)))])
    Card(children) ->
      tagged("card", [#("children", json.preprocessed_array(list.map(children, node_to_json)))])
    CompactCard(width, children) ->
      tagged("compact_card", [
        #("width", json.float(width)),
        #("children", json.preprocessed_array(list.map(children, node_to_json))),
      ])
    InsetRow(children) ->
      tagged("inset_row", [
        #("children", json.preprocessed_array(list.map(children, node_to_json))),
      ])
    LeadTrail(lead, trail) ->
      tagged("lead_trail", [
        #("lead", node_to_json(lead)),
        #("trail", json.preprocessed_array(list.map(trail, node_to_json))),
      ])
    TwoCol(min_col, left, right) -> {
      let fields = [
        #("left", node_to_json(left)),
        #("right", node_to_json(right)),
      ]
      case min_col {
        Some(m) -> tagged("two_col", [#("min_col", json.float(m)), ..fields])
        None -> tagged("two_col", fields)
      }
    }
    Grid(id, cols, rows) ->
      tagged("grid", [
        #("id", json.string(id)),
        #("cols", json.preprocessed_array(list.map(cols, col_to_json))),
        #("rows", json.preprocessed_array(list.map(rows, row_to_json))),
      ])
    Group(children) ->
      tagged("group", [#("children", json.preprocessed_array(list.map(children, node_to_json)))])
  }
}

fn row_to_json(row: Row) -> json.Json {
  case row {
    Cells(cells) ->
      tagged("cells", [#("cells", json.preprocessed_array(list.map(cells, node_to_json)))])
    Values(cells) ->
      tagged("values", [#("cells", json.preprocessed_array(list.map(cells, cell_to_json)))])
  }
}

fn cell_to_json(cell: Cell) -> json.Json {
  case cell {
    Heading(text) -> tagged("heading", [#("text", json.string(text))])
    Text(text) -> tagged("text", [#("text", json.string(text))])
    DimCell(text) -> tagged("dim", [#("text", json.string(text))])
    Warn(text) -> tagged("warn", [#("text", json.string(text))])
    Metric(text) -> tagged("metric", [#("text", json.string(text))])
    MetricDim(text) -> tagged("metric_dim", [#("text", json.string(text))])
    MetricBpsCell(bps) -> tagged("metric_bps", [#("bps", json.float(bps))])
    MetricRateCell(rate) -> tagged("metric_rate", [#("rate", json.float(rate))])
    Nested(node) -> tagged("node", [#("node", node_to_json(node))])
  }
}

fn col_to_json(col: Col) -> json.Json {
  case col {
    Flex -> tagged("flex", [])
    Fixed(px) -> tagged("fixed", [#("px", json.float(px))])
    MetricBps -> tagged("metric_bps", [])
    MetricRate -> tagged("metric_rate", [])
  }
}

fn theme_to_json(theme: ThemeMode) -> json.Json {
  case theme {
    Dark -> json.string("dark")
    Light -> json.string("light")
  }
}

fn icon_to_json(icon: Icon) -> json.Json {
  case icon {
    ThumbsUp -> json.string("thumbs_up")
    ThumbsDown -> json.string("thumbs_down")
    Heart -> json.string("heart")
    Laugh -> json.string("laugh")
    Surprised -> json.string("surprised")
    Frown -> json.string("frown")
    Plus -> json.string("plus")
    Copy -> json.string("copy")
  }
}

fn tagged(kind: String, fields: List(#(String, json.Json))) -> json.Json {
  json.object([#("kind", json.string(kind)), ..fields])
}

// ── Event decode ────────────────────────────────────────────────────────────


pub fn decode_events(raw: String) -> Result(List(Event), json.DecodeError) {
  json.parse(raw, decode.list(event_decoder()))
}

fn event_decoder() -> decode.Decoder(Event) {
  use kind <- decode.field("kind", decode.string)
  case kind {
    "click" -> {
      use id <- decode.field("id", decode.string)
      decode.success(Click(id))
    }
    "check" -> {
      use id <- decode.field("id", decode.string)
      use checked <- decode.field("checked", decode.bool)
      decode.success(Check(id, checked))
    }
    "input" -> {
      use id <- decode.field("id", decode.string)
      use value <- decode.field("value", decode.string)
      decode.success(Input(id, value))
    }
    _ -> decode.failure(Click(""), "click|check|input")
  }
}
