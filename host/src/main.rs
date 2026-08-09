//! Desktop showcase: `just host` or `cargo run --manifest-path host/Cargo.toml`
//!
//! `build.rs` compiles `examples/gleam_fib` with a wasm-capable Gleam; this
//! binary loads that module via wasmtime and calls `gleam_fib__fib`.

mod gleam_guest;

fn main() -> eframe::Result {
    let fib_only = std::env::args().any(|a| a == "--fib-only");

    match gleam_guest::fib(10) {
        Ok(value) => {
            println!("gleam wasm fib(10) = {value}");
            // Safety: single-threaded before eframe; demo reads this for Overview.
            std::env::set_var("VIDYA_GLEAM_FIB", value.to_string());
            if fib_only {
                if value == 55 {
                    return Ok(());
                }
                eprintln!("expected fib(10) = 55, got {value}");
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("gleam wasm fib failed: {err}");
            if fib_only {
                std::process::exit(1);
            }
        }
    }

    vidya_demo::run_desktop()
}
