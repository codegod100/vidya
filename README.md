# Vidya

GNOME/HIG-inspired **theme layer for [egui](https://github.com/emilk/egui)** — no GTK.

Calm charcoal shells, a clear blue accent, soft borders, and a short type/spacing scale. Dark is the default; light is one call away.

**Repo:** [tangled.org/nandi.uk/vidya](https://tangled.org/nandi.uk/vidya)

Screenshots below are **Waydroid** (portrait Android) captures of the demo APK.

<p align="center">
  <img src="docs/screenshots/mobile/vidya-home.png" alt="Vidya — Overview" width="320" />
  <img src="docs/screenshots/mobile/vidya-forms.png" alt="Vidya — Forms (themed checkbox)" width="320" />
  <img src="docs/screenshots/mobile/vidya-actions.png" alt="Vidya — Actions" width="320" />
</p>

<p align="center">
  <img src="docs/screenshots/mobile/vidya-colors.png" alt="Vidya — Palette" width="320" />
  <img src="docs/screenshots/mobile/vidya-type.png" alt="Vidya — Typography" width="320" />
  <img src="docs/screenshots/mobile/vidya-surfaces.png" alt="Vidya — Surfaces" width="320" />
</p>

## Aesthetic

| Layer | Role |
|-------|------|
| Window / view / card / popover | Stacked surfaces (`#242424` → `#383838` in dark) |
| Accent | Adwaita-like blue (`#3584e4`) for primary actions, selection, and checked controls |
| Feedback | Destructive red, success green, warning amber |
| Type | Title 20 · title₂ 16 · body 14 · caption 12 |
| Spacing | 4 · 6 · 12 · 18 · 24 · control height 34 |
| Radius | 6 · 9 · 12 |

`apply()` installs palette + spacing on the egui context so text fields, sliders, and combos inherit the shell. Checkboxes use a dedicated themed control (`checkbox`) with accent fill and a drawn checkmark.

On **Android** (edge-to-edge NativeActivity), call `reserve_system_chrome(ctx, &theme)` once per frame **before** other panels, or use `top_header(ctx, &theme, |ui| { … })` for the app chrome. That reserves the system status bar and gesture/nav band so labels and chips cannot sit under the clock / indicators.

## Demo

An interactive showcase walks through overview, typography, actions, surfaces, palette swatches, and forms (with dark/light toggle).

```bash
nix run                  # apps.default → vidya-demo
nix run .#demo
# or from a remote flake:
nix run git+https://tangled.org/nandi.uk/vidya
```

```bash
nix develop            # rust (+ android target) · just · adb

just waydroid          # in-tree cargo apk → install → launch on Waydroid
just host              # desktop egui window
just install           # same as waydroid (rebuild APK)
just launch            # start installed activity only
just shots             # Waydroid screencaps → docs/screenshots/mobile/
```

`just waydroid` builds **in `android-demo/`** with the nix develop toolchain (no temp/isolated copy).

Sections:

- **Overview** — hero card, design tokens, dark/light pitch  
- **Typography** — type scale samples and hierarchy  
- **Actions** — primary / default / destructive buttons, dialog footer, status pills  
- **Surfaces** — layer stack, card & header frames  
- **Palette** — live semantic swatches (flip the shell to compare)  
- **Forms** — themed inputs, accent checkbox, slider, combo, progress  

### Refreshing screenshots

With a running Waydroid session:

```bash
just install   # rebuild + install APK
just shots     # adb screencap per section → docs/screenshots/mobile/
```

## Use

```toml
[dependencies]
vidya = { git = "https://tangled.org/nandi.uk/vidya" }
# or: vidya = { git = "ssh://git@tangled.org/nandi.uk/vidya" }
egui = "0.31"
```

```rust
use vidya::{apply_dark, checkbox, primary_button, Theme};

apply_dark(ctx);
let th = Theme::dark();
if primary_button(ui, &th, "Open").clicked() {
    // …
}
checkbox(ui, &th, &mut sync, "Sync preferences");
```

## Nix flake

| Output | Role |
|--------|------|
| `apps.default` / `apps.demo` | Aesthetic showcase (`vidya-demo`) |
| `packages.default` / `packages.demo` | Same binary derivation |
| `packages.vidya` | Theme library sources + rlib bundle |
| `devShells.default` | rustc/cargo + egui runtime libs |

```bash
nix run .#demo
nix build .#vidya
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
| `checkbox` | Accent-filled checkbox with drawn checkmark |
| `text_field_singleline` / `text_field_multiline` | Text inputs with field padding |
| `title` / `title_2` / `body` / `dim_label` | Text roles |
| `Theme::header_frame` / `card_frame` / `page_frame` | Layout chrome |
| `Theme::text_edit_margin` | Inner field padding (12×8 default) |
| `reserve_system_chrome` / `system_chrome` | Android status + nav safe areas |
| `top_header` | Header panel with system chrome already reserved |

## License

MIT
