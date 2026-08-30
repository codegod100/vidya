# glimmer-vidya

The **Vidya/egui** backend for [glimmer](https://github.com/jolt-lang/glimmer),
the reactive GUI toolkit for [jolt](https://github.com/jolt-lang/jolt).

glimmer owns the portable half — reactive cells, the component model, the
reconciler — and knows nothing about any toolkit. This project supplies the
other half for a GPU window: Vidya's widgets and theme, painted by egui through
[`../ffi`](../ffi/README.md). Requiring `glimmer-vidya.core` registers it, and
components that render as GTK widgets under
[glimmer-gtk](https://github.com/jolt-lang/glimmer-gtk), and as text under
[glimmer-tui](https://github.com/jolt-lang/glimmer-tui), render here as Vidya.

```clojure
(ns myapp
  (:require [glimmer.ratom :as r :refer [atom]]
            [glimmer.core :as ui]
            [glimmer-vidya.core]))          ; installs this backend

(defn counter []
  (let [count (atom 0)]
    (fn []
      [:page {:max-width 420}
       [:card {}
        [:title {:label "Counter"}]
        [:label {:label (str "Count: " @count)}]
        [:hbox {:spacing 8}
         [:button {:label "- 1" :on-click #(swap! count dec)}]
         [:button {:label "+ 1" :kind :primary :on-click #(swap! count inc)}]
         [:button {:label "reset" :on-click #(reset! count 0)}]]]])))

(defn -main [& _] (ui/run counter :title "counter" :width 480 :height 320))
```

Components, reactive state and reconciliation are documented in glimmer's
README. What follows is the Vidya-specific part.

## How an immediate-mode toolkit holds still

egui has no widgets. It has calls you make every frame, and a reconciler has
nothing to reconcile against — no pointer to patch, nothing to append a child
to. GTK hands glimmer a `GtkButton`; egui hands it nothing at all.

So the widget tree lives one layer down, in Rust. `libvidya` keeps a node arena
behind a second C ABI ([`../ffi/include/vidya_tree.h`](../ffi/include/vidya_tree.h)):
nodes are integer handles, and this backend's `create!` / `apply-props!` /
`append-child!` mutate them. Nothing is painted by those calls. Once a frame,
`vidya_tree_frame` walks the whole tree and emits the egui calls it describes.

Two things follow from putting the tree there rather than here:

* **FFI traffic tracks edits, not frames.** A static UI costs no crossings per
  frame; only what the reconciler actually changed is sent. The alternative —
  keeping the tree in jolt and walking it over the FFI 60 times a second —
  would put the frame rate at the mercy of the reconciler's thread.
* **The closure-shaped parts of egui work.** `ScrollArea` and `Frame` take an
  `FnOnce(&mut Ui)` and keep their `begin`/`end` private, which is why Vidya's
  original push/pop ABI could not scroll a page. Painting from a tree already
  in hand means the recursion *is* the closure.

**Handlers do not cross the boundary.** A jolt closure cannot be a callback in
a library painting at 60fps, so identity travels instead: a node reports that it
was clicked, `glimmer-vidya.core` looks up whose `:on-click` that was, and calls
it on the loop thread. Handlers are held on the jolt side and never sent.

## Requirements

`libvidya` built from [`../ffi`](../ffi), the Rust/egui implementation:

```sh
just ffi            # buck2 → ../build/libvidya.so
```

Then put it on the search path when running anything here:

```sh
LD_LIBRARY_PATH=../build jolt counter
```

`just ffi-android` cross-compiles the same library to
`../build/android/arm64-v8a/libvidya.so` for a 64-bit device — both ABIs this
backend binds are exported there too.

On macOS use `DYLD_LIBRARY_PATH`. Note that this is the one place in the repo
where the two `libvidya` builds are **not** interchangeable: `../raylib`
implements `vidya.h` only, and this backend binds the tree ABI, which is the
Rust build's alone.

## Running

```sh
jolt test       # the suite, headless: no window, display or GPU needed
jolt counter    # the counter above
jolt showcase   # every tag, a keyed task list, an entry, a disabled subtree
jolt smoke      # non-interactive: reconciles under paint, then quits
```

## Hiccup reference

Elements are `[:tag props? & children]`, as everywhere in glimmer. Strings and
numbers become labels, `nil` children are skipped, seqs are spliced.

**Containers**

| tag | holds | notes |
|---|---|---|
| `:window` | many | the root; you rarely name it |
| `:box` / `:hbox` / `:vbox` | many | `:orientation :horizontal\|:vertical`, implied by the tag |
| `:page` | many | scrolling column with page padding; `:max-width` centres it |
| `:scroll` | many | `:orientation :vertical\|:horizontal\|:both`, `:scroll-key` |
| `:card` | many | Vidya's raised surface |
| `:frame` | many | a card with a `:label` as its heading |

**Widgets**

| tag | shows |
|---|---|
| `:label` | body text |
| `:title` / `:title-2` / `:dim-label` | the type scale's other roles |
| `:button` | `:kind :default\|:primary\|:destructive` |
| `:checkbutton` | Vidya's themed checkbox (`:checkbox` is an alias) |
| `:entry` | a text field; `:multiline true` with `:rows` for a box |
| `:separator` | a rule |
| `:spacer` | blank space of `:size` points |
| `:progress` | a bar, `:value` 0.0–1.0, with an optional `:label` |
| `:spinner` | an indeterminate spinner |
| `:status` | a live/offline dot beside a label |

**Common props**

- `:sensitive false` — dims the widget *and its whole subtree*, and takes it out
  of egui's interaction
- `:width-request` — a fixed width. Worth more here than it sounds: immediate
  mode has no natural width for a field, so an `:entry` asks for whatever is
  left and takes the row it shares with a button. This is how you say otherwise.
- `:margin`, `:spacing` — on containers

**Events**

- `:on-click` — button pressed. No args.
- `:on-toggled` — checkbutton clicked. No args.
- `:on-change` — entry text changed. Receives the new text.
- `:on-activate` — Enter pressed in an entry. No args.
- `:on-paste-empty` — Ctrl+V in an entry with no text on the clipboard, which
  is what a copied picture looks like from there. No args; ask
  `clipboard-image-png!` what is actually on it.

As in the other backends, a handler owns the state: `:on-toggled` flips the cell
the component reads, and `:active` comes back down as a prop. A control that
ignores its own event still works — the library wrote the new state into the
node, and the next render either confirms it or overwrites it.

An unrecognized tag is kept rather than refused: it paints as a vertical box, so
a component written against a tag this backend has not grown yet still shows its
children.

## Options for `ui/run`

On top of glimmer's own `:title`, `:width`, `:height` and `:auto-quit-ms`:

| option | |
|---|---|
| `:fps` | frame rate cap, default 60 |
| `:mode` | `:dark` (the default) or `:light` |
| `:font` | path to a TTF/OTF for UI text |

## Timers

The loop wakes every frame anyway, so a timer is a due time and a thunk. Both
run on the loop thread, the only one allowed to touch nodes:

```clojure
(vidya/every! 80 #(swap! tick inc))    ; a spinner, a clock, a progress bar
(vidya/after! 500 #(reset! ready true))
(vidya/cancel! id)
```

`(vidya/quit!)` stops the loop and closes the window.

## Threads

Every node call belongs to the thread that opened the window — the library
enforces it, keeping its state in thread-local storage. glimmer already knows
this: while the loop runs it marshals each component's re-render through the
backend's `schedule`, which queues the work for the next tick. So a `swap!` from
an nREPL worker is safe, and `glimmer.core/on-gui` is there for code that wants
to touch the tree directly.

## Testing

The suite is headless. Node calls need no GL surface — only `vidya_tree_frame`
does — so `jolt test` mounts real components, reconciles them, and reads the
resulting tree back through the same ABI the backend writes it with. It asserts
what was created, what was patched in place, what was replaced, and that a keyed
list reorders its widgets rather than rebuilding them. No window is opened and
no display is required, which is also what makes it run in CI.

Painting is checked separately, by `jolt smoke` under `VIDYA_CAPTURE`:

```sh
VIDYA_CAPTURE=/tmp/frame.ppm LD_LIBRARY_PATH=../build jolt smoke
```

## Limits

* **X11 is preferred on Linux**, for the reason in
  [`../ffi/README.md`](../ffi/README.md): the caller owns the frame loop, and a
  native Wayland surface driven that way stops receiving frame callbacks. Under
  XWayland everything works; fractional scaling follows XWayland's rules.
* **No focus or keyboard navigation of your own.** egui owns focus, tabbing and
  hit testing, so there is nothing here like glimmer-tui's `:keys`, `:on-key` or
  `:autofocus` — and nothing to configure.
* **No `:listbox`, `:table`, `:overlay` or `:paginator` yet.** A list is a
  keyed `:vbox` for now. These are widget-layer work in `../ffi/src/tree.rs`
  plus a tag; nothing in the design is in the way.
