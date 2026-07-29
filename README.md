# Vidya

GNOME/HIG-inspired **theme layer for [egui](https://github.com/emilk/egui)** — no GTK.

Calm charcoal shells, a clear blue accent, soft borders, and a short type/spacing scale. Dark is the default; light is one call away.

**Repo:** [tangled.org/nandi.uk/vidya](https://tangled.org/nandi.uk/vidya)

## Aesthetic

| Layer | Role |
|-------|------|
| Window / view / card / popover | Stacked surfaces (`#242424` → `#383838` in dark) |
| Accent | Adwaita-like blue (`#3584e4`) for primary actions & selection |
| Feedback | Destructive red, success green, warning amber |
| Type | Title 20 · title₂ 16 · body 14 · caption 12 |
| Spacing | 4 · 6 · 12 · 18 · 24 · control height 34 |
| Radius | 6 · 9 · 12 |

Widgets inherit from `apply()` — text fields, checkboxes, sliders, and combos pick up the same visuals without custom paint.

## Demo

An interactive showcase walks through overview, typography, actions, surfaces, palette swatches, and forms (with dark/light toggle):

```bash
cargo run --example demo
```

Sections:

- **Overview** — hero card, design tokens, dark/light pitch  
- **Typography** — type scale samples and hierarchy  
- **Actions** — primary / default / destructive buttons, dialog footer, status pills  
- **Surfaces** — layer stack, card & header frames  
- **Palette** — live semantic swatches (flip the shell to compare)  
- **Forms** — themed egui inputs, slider, combo, progress  

## Use

```toml
[dependencies]
vidya = { git = "https://tangled.org/nandi.uk/vidya" }
# or: vidya = { git = "ssh://git@tangled.org/nandi.uk/vidya" }
egui = "0.31"
```

```rust
use vidya::{apply_dark, primary_button, Theme};

apply_dark(ctx);
let th = Theme::dark();
if primary_button(ui, &th, "Open").clicked() {
    // …
}
```

## Nix flake

```bash
nix build git+https://tangled.org/nandi.uk/vidya
# or local:
nix build .
nix develop
```

As a flake input:

```nix
inputs.vidya.url = "git+https://tangled.org/nandi.uk/vidya";
```

## API

| Item | Role |
|------|------|
| `Theme::dark()` / `light()` | Palette + spacing + type scale |
| `apply` / `apply_dark` / `apply_light` | Install on `egui::Context` |
| `primary_button` / `button` / `destructive_button` | Styled actions |
| `title` / `title_2` / `body` / `dim_label` | Text roles |
| `Theme::header_frame` / `card_frame` / `page_frame` | Layout chrome |

## License

MIT
