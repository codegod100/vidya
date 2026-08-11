//// Full UI designed in Gleam — prints Vidya IR JSON for the Rust host.
////
//// ```bash
//// cd gleam/example && gleam run > /tmp/vidya-ui.json
//// ```

import gleam/io
import gleam/option.{Some}
import vidya.{type App, Dark, Flex, Heart, MetricBps, MetricRate, Plus}

pub fn main() {
  io.println(vidya.encode_app(demo_app()))
}

pub fn demo_app() -> App {
  vidya.App(
    theme: Dark,
    header: Some(
      vidya.hflow([
        vidya.title_2("Vidya"),
        vidya.primary_button("ping", "Ping"),
        vidya.icon_button("add", Plus, "Add"),
      ]),
    ),
    page: vidya.page("gleam-main", [
      vidya.card([
        vidya.title("Designed in Gleam"),
        vidya.body(
          "Every widget below is a Vidya component, composed as a Gleam tree and rendered by egui.",
        ),
        vidya.two_col(
          vidya.vstack([
            vidya.title_2("Preferences"),
            vidya.checkbox("sync", "Sync preferences", True),
            vidya.checkbox("notify", "Desktop notifications", False),
            vidya.text_field("display_name", "nandi"),
            vidya.text_field_multiline("notes", "Hello from Gleam", 3),
          ]),
          vidya.vstack([
            vidya.title_2("Actions"),
            vidya.hflow([
              vidya.primary_button("save", "Save"),
              vidya.button("cancel", "Cancel"),
              vidya.destructive_button("reset", "Reset"),
            ]),
            vidya.hflow([
              vidya.status_dot(True),
              vidya.body("Live"),
              vidya.reaction_chip("heart", "❤️", 3, True),
              vidya.icon(Heart),
            ]),
          ]),
        ),
      ]),
      vidya.card([
        vidya.title_2("Throughput"),
        vidya.grid(
          "procs",
          [Flex, MetricBps, MetricRate],
          [
            vidya.row_values([
              vidya.heading("Name"),
              vidya.heading("Write"),
              vidya.heading("Freq"),
            ]),
            vidya.row_values([
              vidya.cell_text("chrome"),
              vidya.metric_bps(1_250_000.0),
              vidya.metric_rate(42.0),
            ]),
            vidya.row_values([
              vidya.cell_text("gleam"),
              vidya.metric_bps(8200.0),
              vidya.metric_rate(12.0),
            ]),
          ],
        ),
      ]),
      vidya.pack([
        vidya.compact_card(160.0, [vidya.title_2("CPU"), vidya.body("18%")]),
        vidya.compact_card(160.0, [vidya.title_2("Mem"), vidya.body("2.1 GB")]),
        vidya.compact_card(160.0, [
          vidya.title_2("Net"),
          vidya.body("12 Mb/s"),
        ]),
      ]),
    ]),
  )
}
