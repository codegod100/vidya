//! Optional desktop hooks so Overview / Gleam-app can drive Gleam Wasm guests.

use std::sync::OnceLock;

use eframe::egui;
use vidya::{body, button, card, dim_label, hflow, primary_button, title, title_2, Theme};

/// Pure update / query functions backed by the `examples/gleam_gui` Wasm guest.
///
/// Packed model:
/// `((((error*2 + fresh)*8 + op)*1_000_000 + acc)*1_000_000 + entry)`.
/// Op codes: 0=none, 1=+, 2=−, 3=×, 4=÷.
pub struct GleamGuiHooks {
    pub new: Box<dyn Fn() -> Result<i64, String> + Send + Sync>,
    pub digit: Box<dyn Fn(i64, i64) -> Result<i64, String> + Send + Sync>,
    pub op: Box<dyn Fn(i64, i64) -> Result<i64, String> + Send + Sync>,
    pub equals: Box<dyn Fn(i64) -> Result<i64, String> + Send + Sync>,
    pub clear: Box<dyn Fn(i64) -> Result<i64, String> + Send + Sync>,
    pub clear_entry: Box<dyn Fn(i64) -> Result<i64, String> + Send + Sync>,
    pub display: Box<dyn Fn(i64) -> Result<i64, String> + Send + Sync>,
    pub pending_op: Box<dyn Fn(i64) -> Result<i64, String> + Send + Sync>,
    pub errored: Box<dyn Fn(i64) -> Result<i64, String> + Send + Sync>,
}

/// Thin TEA shell: `examples/gleam_shell` owns model/view/update; host walks opcodes.
///
/// View opcodes: `payload * 16 + tag` —
/// title=1, body=2, value=3, button=4, space=5, status=6, header=7,
/// card_open=8, card_close=9.
pub struct GleamShellHooks {
    pub init: Box<dyn Fn() -> Result<i64, String> + Send + Sync>,
    pub update: Box<dyn Fn(i64, i64) -> Result<i64, String> + Send + Sync>,
    pub view_len: Box<dyn Fn(i64) -> Result<i64, String> + Send + Sync>,
    pub view_at: Box<dyn Fn(i64, i64) -> Result<i64, String> + Send + Sync>,
}

static GUI_HOOKS: OnceLock<GleamGuiHooks> = OnceLock::new();
static SHELL_HOOKS: OnceLock<GleamShellHooks> = OnceLock::new();

pub fn install_gleam_gui(hooks: GleamGuiHooks) {
    let _ = GUI_HOOKS.set(hooks);
}

pub fn install_gleam_shell(hooks: GleamShellHooks) {
    let _ = SHELL_HOOKS.set(hooks);
}

pub fn gui_hooks() -> Option<&'static GleamGuiHooks> {
    GUI_HOOKS.get()
}

pub fn shell_hooks() -> Option<&'static GleamShellHooks> {
    SHELL_HOOKS.get()
}

/// Tiny text vocabulary shared with `examples/gleam_shell` text codes.
pub fn shell_text(code: i64) -> &'static str {
    match code {
        1 => "Gleam App",
        2 => "Gleam owns model, view, update, and navigation.",
        3 => "+1",
        4 => "−1",
        5 => "Reset",
        6 => "Ready",
        7 => "Counting up",
        8 => "Counting down",
        9 => "Reset to zero",
        10 => "Home",
        11 => "About",
        12 => "Counter",
        13 => "This whole window is driven by Gleam opcodes. Vidya only \
               applies the theme and paints widgets.",
        14 => "About",
        15 => "Painted by Vidya",
        _ => "?",
    }
}

/// Result of painting one Gleam view frame.
pub struct ShellPaint {
    pub pending_msg: Option<i64>,
    pub error: Option<String>,
}

/// Fetch view opcodes from Gleam and materialize them as Vidya widgets.
pub fn paint_shell_view(
    ui: &mut egui::Ui,
    th: &Theme,
    hooks: &GleamShellHooks,
    model: i64,
) -> ShellPaint {
    let len = match (hooks.view_len)(model) {
        Ok(n) => n.max(0) as usize,
        Err(e) => {
            return ShellPaint {
                pending_msg: None,
                error: Some(e),
            };
        }
    };

    let mut ops = Vec::with_capacity(len);
    for i in 0..len {
        match (hooks.view_at)(model, i as i64) {
            Ok(op) => ops.push(op),
            Err(e) => {
                return ShellPaint {
                    pending_msg: None,
                    error: Some(e),
                };
            }
        }
    }

    let pending_msg = paint_ops(ui, th, &ops, 0, ops.len());
    ShellPaint {
        pending_msg,
        error: None,
    }
}

fn paint_ops(ui: &mut egui::Ui, th: &Theme, ops: &[i64], start: usize, end: usize) -> Option<i64> {
    let mut pending: Option<i64> = None;
    let mut button_row: Vec<(i64, bool, &'static str)> = Vec::new();
    let mut i = start;

    let flush_buttons = |ui: &mut egui::Ui,
                         th: &Theme,
                         row: &mut Vec<(i64, bool, &'static str)>,
                         pending: &mut Option<i64>| {
        if row.is_empty() {
            return;
        }
        hflow(ui, th, |ui| {
            for &(msg, primary, label) in row.iter() {
                let clicked = if primary {
                    primary_button(ui, th, label).clicked()
                } else {
                    button(ui, th, label).clicked()
                };
                if clicked {
                    *pending = Some(msg);
                }
            }
        });
        row.clear();
    };

    while i < end {
        let op = ops[i];
        let tag = op % 16;
        let payload = op / 16;
        match tag {
            1 => {
                flush_buttons(ui, th, &mut button_row, &mut pending);
                title_2(ui, th, shell_text(payload));
                i += 1;
            }
            2 => {
                flush_buttons(ui, th, &mut button_row, &mut pending);
                body(ui, th, shell_text(payload));
                i += 1;
            }
            3 => {
                flush_buttons(ui, th, &mut button_row, &mut pending);
                title_2(ui, th, &payload.to_string());
                i += 1;
            }
            4 => {
                let label_code = payload % 256;
                let msg = (payload / 256) % 256;
                let primary = (payload / 65_536) % 2 == 1;
                button_row.push((msg, primary, shell_text(label_code)));
                i += 1;
            }
            5 => {
                flush_buttons(ui, th, &mut button_row, &mut pending);
                let space = match payload {
                    0 => th.spacing.xs,
                    1 => th.spacing.sm,
                    _ => th.spacing.md,
                };
                ui.add_space(space);
                i += 1;
            }
            6 => {
                flush_buttons(ui, th, &mut button_row, &mut pending);
                dim_label(ui, th, shell_text(payload));
                i += 1;
            }
            7 => {
                flush_buttons(ui, th, &mut button_row, &mut pending);
                title(ui, th, shell_text(payload));
                i += 1;
            }
            8 => {
                flush_buttons(ui, th, &mut button_row, &mut pending);
                let close = find_card_end(ops, i + 1, end);
                let inner_pending = {
                    let mut p = None;
                    card(ui, th, |ui| {
                        p = paint_ops(ui, th, ops, i + 1, close);
                    });
                    p
                };
                if inner_pending.is_some() {
                    pending = inner_pending;
                }
                i = close.min(end.saturating_sub(1)) + 1;
                if close >= end {
                    break;
                }
            }
            9 => {
                // Unmatched close — stop this range.
                flush_buttons(ui, th, &mut button_row, &mut pending);
                break;
            }
            _ => {
                i += 1;
            }
        }
    }
    flush_buttons(ui, th, &mut button_row, &mut pending);
    pending
}

fn find_card_end(ops: &[i64], from: usize, end: usize) -> usize {
    let mut depth = 1i32;
    let mut i = from;
    while i < end {
        let tag = ops[i] % 16;
        if tag == 8 {
            depth += 1;
        } else if tag == 9 {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
        i += 1;
    }
    end
}
