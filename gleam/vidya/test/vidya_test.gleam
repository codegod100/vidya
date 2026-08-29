import gleam/string
import gleeunit
import vidya

pub fn main() {
  gleeunit.main()
}

pub fn encode_primary_button_test() {
  let json = vidya.encode_node(vidya.primary_button("save", "Save"))
  assert string.contains(json, "\"kind\":\"primary_button\"")
  assert string.contains(json, "\"id\":\"save\"")
}

pub fn encode_app_sections_test() {
  let app =
    vidya.app(
      vidya.page("main", [
        vidya.card([
          vidya.title("Hello"),
          vidya.checkbox("sync", "Sync", True),
          vidya.text_field("name", "nandi"),
          vidya.hflow([
            vidya.primary_button("save", "Save"),
            vidya.destructive_button("reset", "Reset"),
            vidya.status_dot(True),
          ]),
        ]),
        vidya.grid(
          "metrics",
          [vidya.Flex, vidya.MetricBps, vidya.MetricRate],
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
          ],
        ),
      ]),
    )
  let json = vidya.encode_app(app)
  assert string.contains(json, "\"kind\":\"card\"")
  assert string.contains(json, "\"kind\":\"grid\"")
  assert string.contains(json, "\"kind\":\"metric_bps\"")
}

pub fn decode_events_test() {
  let assert Ok(events) =
    vidya.decode_events(
      "[{\"kind\":\"click\",\"id\":\"save\"},{\"kind\":\"check\",\"id\":\"sync\",\"checked\":false},{\"kind\":\"input\",\"id\":\"name\",\"value\":\"x\"}]",
    )
  assert events
    == [
      vidya.Click("save"),
      vidya.Check("sync", False),
      vidya.Input("name", "x"),
    ]
}
