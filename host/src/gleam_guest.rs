//! Load Gleam-compiled Wasm guests and call typed exports (`module__function`).
//!
//! Gleam `Int` ↔ Wasm `i64`. Guests omit `main` so Instantiation needs no imports.
//! Calculator + shell each keep one Engine / Module / Store / Instance for the process.

use std::sync::{Mutex, OnceLock};

use wasmtime::{Engine, Instance, Module, Store, TypedFunc};

const FIB_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gleam_fib.wasm"));
const GUI_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gleam_gui.wasm"));
const SHELL_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gleam_shell.wasm"));

const FIB_EXPORT: &str = "gleam_fib__fib";

struct GuiSession {
    store: Store<()>,
    new: TypedFunc<(), i64>,
    digit: TypedFunc<(i64, i64), i64>,
    op: TypedFunc<(i64, i64), i64>,
    equals: TypedFunc<i64, i64>,
    clear: TypedFunc<i64, i64>,
    clear_entry: TypedFunc<i64, i64>,
    display: TypedFunc<i64, i64>,
    pending_op: TypedFunc<i64, i64>,
    errored: TypedFunc<i64, i64>,
}

struct ShellSession {
    store: Store<()>,
    init: TypedFunc<(), i64>,
    update: TypedFunc<(i64, i64), i64>,
    view_len: TypedFunc<i64, i64>,
    view_at: TypedFunc<(i64, i64), i64>,
}

static GUI: OnceLock<Mutex<GuiSession>> = OnceLock::new();
static SHELL: OnceLock<Mutex<ShellSession>> = OnceLock::new();

fn gui() -> Result<&'static Mutex<GuiSession>, String> {
    if let Some(g) = GUI.get() {
        return Ok(g);
    }
    let session = GuiSession::load()?;
    let _ = GUI.set(Mutex::new(session));
    GUI.get()
        .ok_or_else(|| "gui session missing after init".into())
}

fn shell() -> Result<&'static Mutex<ShellSession>, String> {
    if let Some(s) = SHELL.get() {
        return Ok(s);
    }
    let session = ShellSession::load()?;
    let _ = SHELL.set(Mutex::new(session));
    SHELL
        .get()
        .ok_or_else(|| "shell session missing after init".into())
}

impl GuiSession {
    fn load() -> Result<Self, String> {
        let engine = Engine::default();
        let module =
            Module::new(&engine, GUI_WASM).map_err(|e| format!("parse gleam_gui.wasm: {e}"))?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| format!("instantiate gleam_gui.wasm: {e}"))?;

        let get0 = |store: &mut Store<()>, name: &str| -> Result<TypedFunc<(), i64>, String> {
            instance
                .get_typed_func(store, name)
                .map_err(|e| format!("export {name}: {e}"))
        };
        let get1 = |store: &mut Store<()>, name: &str| -> Result<TypedFunc<i64, i64>, String> {
            instance
                .get_typed_func(store, name)
                .map_err(|e| format!("export {name}: {e}"))
        };
        let get2 =
            |store: &mut Store<()>, name: &str| -> Result<TypedFunc<(i64, i64), i64>, String> {
                instance
                    .get_typed_func(store, name)
                    .map_err(|e| format!("export {name}: {e}"))
            };

        Ok(Self {
            new: get0(&mut store, "gleam_gui__new")?,
            digit: get2(&mut store, "gleam_gui__digit")?,
            op: get2(&mut store, "gleam_gui__op")?,
            equals: get1(&mut store, "gleam_gui__equals")?,
            clear: get1(&mut store, "gleam_gui__clear")?,
            clear_entry: get1(&mut store, "gleam_gui__clear_entry")?,
            display: get1(&mut store, "gleam_gui__display")?,
            pending_op: get1(&mut store, "gleam_gui__pending_op")?,
            errored: get1(&mut store, "gleam_gui__errored")?,
            store,
        })
    }
}

impl ShellSession {
    fn load() -> Result<Self, String> {
        let engine = Engine::default();
        let module = Module::new(&engine, SHELL_WASM)
            .map_err(|e| format!("parse gleam_shell.wasm: {e}"))?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| format!("instantiate gleam_shell.wasm: {e}"))?;

        let get0 = |store: &mut Store<()>, name: &str| -> Result<TypedFunc<(), i64>, String> {
            instance
                .get_typed_func(store, name)
                .map_err(|e| format!("export {name}: {e}"))
        };
        let get1 = |store: &mut Store<()>, name: &str| -> Result<TypedFunc<i64, i64>, String> {
            instance
                .get_typed_func(store, name)
                .map_err(|e| format!("export {name}: {e}"))
        };
        let get2 =
            |store: &mut Store<()>, name: &str| -> Result<TypedFunc<(i64, i64), i64>, String> {
                instance
                    .get_typed_func(store, name)
                    .map_err(|e| format!("export {name}: {e}"))
            };

        Ok(Self {
            init: get0(&mut store, "gleam_shell__init")?,
            update: get2(&mut store, "gleam_shell__update")?,
            view_len: get1(&mut store, "gleam_shell__view_len")?,
            view_at: get2(&mut store, "gleam_shell__view_at")?,
            store,
        })
    }
}

pub fn fib(n: i64) -> Result<i64, String> {
    call_i64_i64(FIB_WASM, "gleam_fib.wasm", FIB_EXPORT, n)
}

pub fn gui_new() -> Result<i64, String> {
    with_gui(|g| {
        g.new
            .call(&mut g.store, ())
            .map_err(|e| format!("call gleam_gui__new(): {e}"))
    })
}

pub fn gui_digit(model: i64, d: i64) -> Result<i64, String> {
    let d = d.clamp(0, 9);
    with_gui(|g| {
        g.digit
            .call(&mut g.store, (model, d))
            .map_err(|e| format!("call gleam_gui__digit({model}, {d}): {e}"))
    })
}

pub fn gui_op(model: i64, opcode: i64) -> Result<i64, String> {
    let opcode = if (1..=4).contains(&opcode) { opcode } else { 0 };
    with_gui(|g| {
        g.op
            .call(&mut g.store, (model, opcode))
            .map_err(|e| format!("call gleam_gui__op({model}, {opcode}): {e}"))
    })
}

pub fn gui_equals(model: i64) -> Result<i64, String> {
    with_gui(|g| {
        g.equals
            .call(&mut g.store, model)
            .map_err(|e| format!("call gleam_gui__equals({model}): {e}"))
    })
}

pub fn gui_clear(model: i64) -> Result<i64, String> {
    with_gui(|g| {
        g.clear
            .call(&mut g.store, model)
            .map_err(|e| format!("call gleam_gui__clear({model}): {e}"))
    })
}

pub fn gui_clear_entry(model: i64) -> Result<i64, String> {
    with_gui(|g| {
        g.clear_entry
            .call(&mut g.store, model)
            .map_err(|e| format!("call gleam_gui__clear_entry({model}): {e}"))
    })
}

pub fn gui_display(model: i64) -> Result<i64, String> {
    with_gui(|g| {
        g.display
            .call(&mut g.store, model)
            .map_err(|e| format!("call gleam_gui__display({model}): {e}"))
    })
}

pub fn gui_pending_op(model: i64) -> Result<i64, String> {
    with_gui(|g| {
        g.pending_op
            .call(&mut g.store, model)
            .map_err(|e| format!("call gleam_gui__pending_op({model}): {e}"))
    })
}

pub fn gui_errored(model: i64) -> Result<i64, String> {
    with_gui(|g| {
        g.errored
            .call(&mut g.store, model)
            .map_err(|e| format!("call gleam_gui__errored({model}): {e}"))
    })
}

pub fn shell_init() -> Result<i64, String> {
    with_shell(|s| {
        s.init
            .call(&mut s.store, ())
            .map_err(|e| format!("call gleam_shell__init(): {e}"))
    })
}

pub fn shell_update(model: i64, msg: i64) -> Result<i64, String> {
    with_shell(|s| {
        s.update
            .call(&mut s.store, (model, msg))
            .map_err(|e| format!("call gleam_shell__update({model}, {msg}): {e}"))
    })
}

pub fn shell_view_len(model: i64) -> Result<i64, String> {
    with_shell(|s| {
        s.view_len
            .call(&mut s.store, model)
            .map_err(|e| format!("call gleam_shell__view_len({model}): {e}"))
    })
}

pub fn shell_view_at(model: i64, i: i64) -> Result<i64, String> {
    with_shell(|s| {
        s.view_at
            .call(&mut s.store, (model, i))
            .map_err(|e| format!("call gleam_shell__view_at({model}, {i}): {e}"))
    })
}

fn with_gui<T>(f: impl FnOnce(&mut GuiSession) -> Result<T, String>) -> Result<T, String> {
    let mutex = gui()?;
    let mut g = mutex
        .lock()
        .map_err(|_| "gleam gui session lock poisoned".to_string())?;
    f(&mut g)
}

fn with_shell<T>(f: impl FnOnce(&mut ShellSession) -> Result<T, String>) -> Result<T, String> {
    let mutex = shell()?;
    let mut s = mutex
        .lock()
        .map_err(|_| "gleam shell session lock poisoned".to_string())?;
    f(&mut s)
}

fn call_i64_i64(wasm: &[u8], label: &str, export: &str, n: i64) -> Result<i64, String> {
    let engine = Engine::default();
    let module = Module::new(&engine, wasm).map_err(|e| format!("parse {label}: {e}"))?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])
        .map_err(|e| format!("instantiate {label}: {e}"))?;
    let func = instance
        .get_typed_func::<i64, i64>(&mut store, export)
        .map_err(|e| format!("export {export}: {e}"))?;
    func.call(&mut store, n)
        .map_err(|e| format!("call {export}({n}): {e}"))
}
