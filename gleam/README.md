# Gleam ↔ Vidya

Declarative UI for Vidya. Compose screens in Gleam; Rust renders them with egui.

| Path | Role |
|------|------|
| [`vidya/`](vidya/) | Gleam package — builders for every Vidya component |
| [`example/`](example/) | Full sample UI (`gleam run` → JSON) |
| [`example/demo_app.json`](example/demo_app.json) | Fixture parsed by Rust `vidya::gleam` tests |

```bash
just gleam-test
just gleam-example
```

See the root README **Gleam** section for the component mapping table and Rust host snippet.
