# gleam_shell

Gleam **TEA** mini-app (`init` / `update` / `view_len` / `view_at`) compiled to
Wasm and hosted by a **thin Vidya/egui shell**. Gleam owns the model, navigation
(Home / About), and the declarative view; Rust only walks packed opcodes and
forwards button msgs.

Requires a wasm-capable Gleam binary (branch `wasm` of
[nandi.uk/gleam](https://tangled.org/nandi.uk/gleam)):

```bash
export GLEAM=~/code/gleam/target/debug/gleam
just gleam-shell   # build + scripted smoke (no window)
just gleam-app     # whole window — Gleam owns the UI tree
just host          # aesthetic showcase (shell still appears as an Overview card)
```

## View opcodes

Each `view_at(model, i)` Int is `payload * 16 + tag`:

| Tag | Name       | Payload |
|-----|------------|---------|
| 1   | title      | text code |
| 2   | body       | text code |
| 3   | value      | display number (count) |
| 4   | button     | `(primary << 16) \| (msg_id << 8) \| label_code` |
| 5   | space      | 0=xs · 1=sm · 2=md |
| 6   | status     | text code |
| 7   | header     | text code (app chrome title) |
| 8   | card_open  | 0 |
| 9   | card_close | 0 |

Text codes map to a tiny host vocabulary (`Gleam App`, `Home`, `+1`, …).
Msgs: `0=Inc`, `1=Dec`, `2=Reset`, `3=GoHome`, `4=GoAbout`.

**Note:** module-level `const` is not lowered by the Wasm MIR yet — numeric
codes are inlined (documented in comments).

## Model

```
model = screen * 10_000_000 + last_action * 1_000_000 + count
```

`screen`: 0=Home, 1=About. `last_action`: 0=none, 1=inc, 2=dec, 3=reset.
Count stays in `0..999_999`.
