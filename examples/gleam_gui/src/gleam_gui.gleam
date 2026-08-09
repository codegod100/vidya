//// Toy integer calculator for the Vidya desktop host.
////
//// Exports: gleam_gui__{new,digit,op,equals,clear,clear_entry,display,pending_op,errored}
//// Int ↔ Wasm i64. No `main` (no WASI).
////
//// Model: ((((error * 2 + fresh) * 8 + op) * 1_000_000 + acc) * 1_000_000 + entry
//// Ops: 0=none, 1=+, 2=−, 3=×, 4=÷. Values 0..999_999.

pub fn new() -> Int {
  pack(0, 0, 0, 1, 0)
}

pub fn display(model: Int) -> Int {
  entry_of(model)
}

pub fn pending_op(model: Int) -> Int {
  op_of(model)
}

pub fn errored(model: Int) -> Int {
  error_of(model)
}

pub fn clear(model: Int) -> Int {
  let _ = model
  new()
}

pub fn clear_entry(model: Int) -> Int {
  pack(0, acc_of(model), op_of(model), 1, 0)
}

pub fn digit(model: Int, d: Int) -> Int {
  digit_dispatch(error_of(model), model, d)
}

pub fn op(model: Int, opcode: Int) -> Int {
  op_dispatch(error_of(model), model, opcode)
}

pub fn equals(model: Int) -> Int {
  equals_dispatch(error_of(model), model)
}

fn digit_dispatch(error: Int, model: Int, digit: Int) -> Int {
  case error {
    1 -> model
    _ -> digit_fresh(fresh_of(model), model, digit)
  }
}

fn digit_fresh(fresh: Int, model: Int, digit: Int) -> Int {
  case fresh {
    1 -> pack(digit, acc_of(model), op_of(model), 0, 0)
    _ -> append_digit(model, digit)
  }
}

fn append_digit(model: Int, digit: Int) -> Int {
  let entry = entry_of(model)
  let next = entry * 10 + digit
  case next > 999_999 {
    True -> pack(entry, acc_of(model), op_of(model), fresh_of(model), 1)
    False -> pack(next, acc_of(model), op_of(model), 0, 0)
  }
}

fn op_dispatch(error: Int, model: Int, opcode: Int) -> Int {
  case error {
    1 -> model
    _ -> op_code(model, opcode)
  }
}

fn op_code(model: Int, opcode: Int) -> Int {
  case opcode {
    0 -> model
    _ -> finish(model, opcode)
  }
}

fn equals_dispatch(error: Int, model: Int) -> Int {
  case error {
    1 -> model
    _ -> finish(model, 0)
  }
}

fn finish(model: Int, next_op: Int) -> Int {
  finish_fresh(fresh_of(model), model, next_op)
}

fn finish_fresh(fresh: Int, model: Int, next_op: Int) -> Int {
  case fresh {
    1 -> pack(entry_of(model), entry_of(model), next_op, 1, 0)
    _ -> finish_op(op_of(model), acc_of(model), entry_of(model), next_op)
  }
}

fn finish_op(pending: Int, acc: Int, right: Int, next_op: Int) -> Int {
  case pending {
    1 -> finish_add(acc, right, next_op)
    2 -> finish_sub(acc, right, next_op)
    3 -> finish_mul(acc, right, next_op)
    4 -> finish_div(acc, right, next_op)
    _ -> pack(right, right, next_op, 1, 0)
  }
}

fn finish_add(acc: Int, right: Int, next_op: Int) -> Int {
  let value = acc + right
  case value > 999_999 {
    True -> pack(right, acc, next_op, 0, 1)
    False -> pack(value, value, next_op, 1, 0)
  }
}

fn finish_sub(acc: Int, right: Int, next_op: Int) -> Int {
  let value = acc - right
  case 0 > value {
    True -> pack(right, acc, next_op, 0, 1)
    False -> pack(value, value, next_op, 1, 0)
  }
}

fn finish_mul(acc: Int, right: Int, next_op: Int) -> Int {
  let value = acc * right
  case value > 999_999 {
    True -> pack(right, acc, next_op, 0, 1)
    False -> pack(value, value, next_op, 1, 0)
  }
}

fn finish_div(acc: Int, right: Int, next_op: Int) -> Int {
  case right {
    0 -> pack(right, acc, next_op, 0, 1)
    _ -> pack(acc / right, acc / right, next_op, 1, 0)
  }
}

fn pack(entry: Int, acc: Int, op: Int, fresh: Int, error: Int) -> Int {
  {{{{error * 2 + fresh} * 8 + op} * 1_000_000 + acc} * 1_000_000 + entry}
}

fn entry_of(model: Int) -> Int {
  model % 1_000_000
}

fn acc_of(model: Int) -> Int {
  let rest = model / 1_000_000
  rest % 1_000_000
}

fn op_of(model: Int) -> Int {
  let rest = model / 1_000_000
  let rest = rest / 1_000_000
  rest % 8
}

fn fresh_of(model: Int) -> Int {
  let rest = model / 1_000_000
  let rest = rest / 1_000_000
  let rest = rest / 8
  rest % 2
}

fn error_of(model: Int) -> Int {
  let rest = model / 1_000_000
  let rest = rest / 1_000_000
  let rest = rest / 8
  rest / 2
}
