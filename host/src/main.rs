//! Desktop host: `just host` (showcase) · `just gleam-app` (whole Gleam window)
//!
//! `build.rs` compiles Gleam Wasm guests; this binary loads them via wasmtime
//! (`gleam_fib__fib`, `gleam_gui__*`, `gleam_shell__{init,update,view_*}`).

mod gleam_guest;

fn main() -> eframe::Result {
    let fib_only = std::env::args().any(|a| a == "--fib-only");
    let gui_only = std::env::args().any(|a| a == "--gui-only");
    let shell_only = std::env::args().any(|a| a == "--shell-only");
    let gleam_app = std::env::args().any(|a| a == "--gleam-app");
    let showcase = std::env::args().any(|a| a == "--showcase");

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

    vidya_demo::install_gleam_gui(vidya_demo::GleamGuiHooks {
        new: Box::new(gleam_guest::gui_new),
        digit: Box::new(gleam_guest::gui_digit),
        op: Box::new(gleam_guest::gui_op),
        equals: Box::new(gleam_guest::gui_equals),
        clear: Box::new(gleam_guest::gui_clear),
        clear_entry: Box::new(gleam_guest::gui_clear_entry),
        display: Box::new(gleam_guest::gui_display),
        pending_op: Box::new(gleam_guest::gui_pending_op),
        errored: Box::new(gleam_guest::gui_errored),
    });

    vidya_demo::install_gleam_shell(vidya_demo::GleamShellHooks {
        init: Box::new(gleam_guest::shell_init),
        update: Box::new(gleam_guest::shell_update),
        view_len: Box::new(gleam_guest::shell_view_len),
        view_at: Box::new(gleam_guest::shell_view_at),
    });

    match smoke_gui() {
        Ok(summary) => {
            println!("gleam wasm gui smoke → {summary}");
            if gui_only {
                return Ok(());
            }
        }
        Err(err) => {
            eprintln!("gleam wasm gui failed: {err}");
            if gui_only {
                std::process::exit(1);
            }
        }
    }

    match smoke_shell() {
        Ok(summary) => {
            println!("gleam wasm shell smoke → {summary}");
            if shell_only {
                return Ok(());
            }
        }
        Err(err) => {
            eprintln!("gleam wasm shell failed: {err}");
            if shell_only {
                std::process::exit(1);
            }
        }
    }

    if fib_only || gui_only || shell_only {
        return Ok(());
    }

    // Whole-window Gleam app (primary “all Gleam” experience).
    if gleam_app && !showcase {
        return vidya_demo::run_gleam_app();
    }

    // Aesthetic showcase (Overview still embeds a Gleam shell card).
    vidya_demo::run_desktop()
}

/// Scripted calculator: 12 + 34 = 46, then clear.
fn smoke_gui() -> Result<String, String> {
    let mut m = gleam_guest::gui_new()?;
    expect_display(m, 0, 0, 0, "new")?;

    m = gleam_guest::gui_digit(m, 1)?;
    m = gleam_guest::gui_digit(m, 2)?;
    expect_display(m, 12, 0, 0, "12")?;

    m = gleam_guest::gui_op(m, 1)?; // +
    expect_display(m, 12, 1, 0, "pending +")?;

    m = gleam_guest::gui_digit(m, 3)?;
    m = gleam_guest::gui_digit(m, 4)?;
    expect_display(m, 34, 1, 0, "34")?;

    m = gleam_guest::gui_equals(m)?;
    expect_display(m, 46, 0, 0, "12+34")?;

    m = gleam_guest::gui_clear(m)?;
    expect_display(m, 0, 0, 0, "clear")?;

    // Division / multiply on the same long-lived instance
    m = gleam_guest::gui_digit(m, 8)?;
    m = gleam_guest::gui_op(m, 3)?; // ×
    m = gleam_guest::gui_digit(m, 7)?;
    m = gleam_guest::gui_equals(m)?;
    expect_display(m, 56, 0, 0, "8*7")?;

    Ok(format!(
        "display={} pending={} err={}",
        gleam_guest::gui_display(m)?,
        gleam_guest::gui_pending_op(m)?,
        gleam_guest::gui_errored(m)?,
    ))
}

/// Scripted TEA mini-app: Inc×2, Dec, Reset, About, Home — assert VALUE + nav.
fn smoke_shell() -> Result<String, String> {
    let mut m = gleam_guest::shell_init()?;
    expect_shell_value(m, 0, "init")?;

    m = gleam_guest::shell_update(m, 0)?; // Inc
    expect_shell_value(m, 1, "inc")?;

    m = gleam_guest::shell_update(m, 0)?; // Inc
    expect_shell_value(m, 2, "inc2")?;

    m = gleam_guest::shell_update(m, 1)?; // Dec
    expect_shell_value(m, 1, "dec")?;

    m = gleam_guest::shell_update(m, 2)?; // Reset
    expect_shell_value(m, 0, "reset")?;

    let len = gleam_guest::shell_view_len(m)?;
    if len != 14 {
        return Err(format!("home view_len: expected 14, got {len}"));
    }

    // Header opcode at index 0: tag=7, text_code=1 ("Gleam App")
    let header = gleam_guest::shell_view_at(m, 0)?;
    if header % 16 != 7 || header / 16 != 1 {
        return Err(format!(
            "header opcode: expected tag=7 code=1, got {header}"
        ));
    }

    // Navigate to About — no VALUE opcode; view_len shrinks.
    m = gleam_guest::shell_update(m, 4)?; // GoAbout
    let about_len = gleam_guest::shell_view_len(m)?;
    if about_len != 9 {
        return Err(format!("about view_len: expected 9, got {about_len}"));
    }
    if shell_value(m).is_ok() {
        return Err("about screen should not emit TAG_VALUE".into());
    }

    m = gleam_guest::shell_update(m, 3)?; // GoHome
    expect_shell_value(m, 0, "home again")?;

    Ok(format!(
        "value={} home_view_len={} about_view_len={about_len}",
        shell_value(m)?,
        gleam_guest::shell_view_len(m)?,
    ))
}

fn shell_value(model: i64) -> Result<i64, String> {
    let len = gleam_guest::shell_view_len(model)?;
    for i in 0..len {
        let op = gleam_guest::shell_view_at(model, i)?;
        if op % 16 == 3 {
            return Ok(op / 16);
        }
    }
    Err("no TAG_VALUE in view".into())
}

fn expect_shell_value(model: i64, expected: i64, step: &str) -> Result<(), String> {
    let got = shell_value(model)?;
    if got != expected {
        return Err(format!("{step}: expected value={expected}, got {got}"));
    }
    Ok(())
}

fn expect_display(
    model: i64,
    display: i64,
    pending: i64,
    errored: i64,
    step: &str,
) -> Result<(), String> {
    let got_d = gleam_guest::gui_display(model)?;
    let got_p = gleam_guest::gui_pending_op(model)?;
    let got_e = gleam_guest::gui_errored(model)?;
    if got_d != display || got_p != pending || got_e != errored {
        return Err(format!(
            "{step}: expected display={display} pending={pending} err={errored}, \
             got display={got_d} pending={got_p} err={got_e}"
        ));
    }
    Ok(())
}
