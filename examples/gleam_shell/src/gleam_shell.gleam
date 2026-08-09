//// Gleam-owned mini-app (model + update + view) for a thin Vidya shell.
////
//// Exports: gleam_shell__{init,update,view_len,view_at}
//// Int ↔ Wasm i64. No `main` (no WASI). No Strings across the ABI.
//// Avoid module `const` (Wasm MIR does not lower ModuleConstant yet).
////
//// Model: screen * 10_000_000 + last_action * 1_000_000 + count
////   count 0..999_999
////   last_action 0=none · 1=inc · 2=dec · 3=reset
////   screen 0=Home · 1=About
////
//// Msg: 0=Inc · 1=Dec · 2=Reset · 3=GoHome · 4=GoAbout
////
//// View: flat opcode buffer indexed by `view_at(model, i)`.
////   low 4 bits = tag · remaining bits = payload
////   TAG_TITLE=1     payload=text_code
////   TAG_BODY=2      payload=text_code
////   TAG_VALUE=3     payload=display int (count)
////   TAG_BUTTON=4    payload=(primary << 16) | (msg_id << 8) | label_code
////   TAG_SPACE=5     payload=0=xs · 1=sm · 2=md
////   TAG_STATUS=6    payload=text_code
////   TAG_HEADER=7    payload=text_code
////   TAG_CARD_OPEN=8 payload=0
////   TAG_CARD_CLOSE=9 payload=0
//// Host maps text_code → &str (tiny vocabulary below).
////
//// Text codes:
////   1=Gleam App  2=home body  3=+1  4=−1  5=Reset
////   6=Ready  7=Counting up  8=Counting down  9=Reset to zero
////   10=Home  11=About  12=Counter  13=about body  14=About
////   15=Painted by Vidya

// --- TEA -------------------------------------------------------------------

pub fn init() -> Int {
  pack(0, 0, 0)
}

pub fn update(model: Int, msg: Int) -> Int {
  case msg {
    0 -> pack(bump(count_of(model)), 1, screen_of(model))
    1 -> pack(drop(count_of(model)), 2, screen_of(model))
    2 -> pack(0, 3, screen_of(model))
    3 -> pack(count_of(model), last_of(model), 0)
    4 -> pack(count_of(model), last_of(model), 1)
    _ -> model
  }
}

/// Number of opcodes in `view` for this model.
pub fn view_len(model: Int) -> Int {
  case screen_of(model) {
    1 -> 9
    _ -> 14
  }
}

/// Opcode at index `i` — full-window mini-app layout (Home / About).
pub fn view_at(model: Int, i: Int) -> Int {
  case screen_of(model) {
    1 -> about_at(i)
    _ -> home_at(i, count_of(model), last_of(model))
  }
}

// --- Screens ---------------------------------------------------------------

/// Home: header + nav + card(counter + actions + status).
fn home_at(i: Int, count: Int, last: Int) -> Int {
  case i {
    0 -> header(1)
    // button(primary, msg_home=3, txt_home=10)
    1 -> button(1, 3, 10)
    // button(default, msg_about=4, txt_about=11)
    2 -> button(0, 4, 11)
    3 -> space(2)
    4 -> card_open()
    // title Counter
    5 -> title(12)
    // body home
    6 -> body(2)
    7 -> value(count)
    8 -> space(2)
    // +1 / −1 / Reset
    9 -> button(1, 0, 3)
    10 -> button(0, 1, 4)
    11 -> button(0, 2, 5)
    12 -> status(status_code(last))
    13 -> card_close()
    _ -> 0
  }
}

/// About: header + nav + card(about copy).
fn about_at(i: Int) -> Int {
  case i {
    0 -> header(1)
    // Home default · About primary
    1 -> button(0, 3, 10)
    2 -> button(1, 4, 11)
    3 -> space(2)
    4 -> card_open()
    5 -> title(14)
    6 -> body(13)
    7 -> status(15)
    8 -> card_close()
    _ -> 0
  }
}

// --- View DSL helpers (declarative constructors → packed Int) --------------

fn header(code: Int) -> Int {
  // tag_header = 7
  pack_op(7, code)
}

fn title(code: Int) -> Int {
  // tag_title = 1
  pack_op(1, code)
}

fn body(code: Int) -> Int {
  // tag_body = 2
  pack_op(2, code)
}

fn value(n: Int) -> Int {
  // tag_value = 3
  pack_op(3, n)
}

fn button(primary: Int, msg: Int, label: Int) -> Int {
  // tag_button = 4 · payload = primary*65536 + msg*256 + label
  pack_op(4, primary * 65_536 + msg * 256 + label)
}

fn space(sz: Int) -> Int {
  // tag_space = 5
  pack_op(5, sz)
}

fn status(code: Int) -> Int {
  // tag_status = 6
  pack_op(6, code)
}

fn card_open() -> Int {
  // tag_card_open = 8
  pack_op(8, 0)
}

fn card_close() -> Int {
  // tag_card_close = 9
  pack_op(9, 0)
}

fn pack_op(tag: Int, payload: Int) -> Int {
  payload * 16 + tag
}

fn status_code(last: Int) -> Int {
  // txt_ready=6, txt_up=7, txt_down=8, txt_zero=9
  case last {
    1 -> 7
    2 -> 8
    3 -> 9
    _ -> 6
  }
}

// --- Model packing ---------------------------------------------------------

fn pack(count: Int, last: Int, screen: Int) -> Int {
  screen * 10_000_000 + last * 1_000_000 + count
}

fn count_of(model: Int) -> Int {
  model % 1_000_000
}

fn last_of(model: Int) -> Int {
  model / 1_000_000 % 10
}

fn screen_of(model: Int) -> Int {
  model / 10_000_000
}

fn bump(n: Int) -> Int {
  case n > 999_998 {
    True -> 999_999
    False -> n + 1
  }
}

fn drop(n: Int) -> Int {
  case n {
    0 -> 0
    _ -> n - 1
  }
}
