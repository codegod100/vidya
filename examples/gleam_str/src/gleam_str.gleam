//// String ABI smoke guest for the Vidya desktop host.
////
//// Exports (Wasm): gleam_str__{hello,roundtrip,greet,same}
//// String ↔ i32 pointer to `{ len: u32, data: [u8; len] }` in linear memory.
//// No `main` (no WASI) so Instantiation needs no imports.

pub fn hello() -> String {
  "hello from gleam"
}

pub fn roundtrip(s: String) -> String {
  s
}

pub fn greet(name: String) -> String {
  "hello, " <> name <> "!"
}

pub fn same(a: String, b: String) -> Bool {
  a == b
}
