/*
 * Vidya's retained-tree ABI — the reactive half.
 *
 * `raylib/include/vidya.h` is push/pop: the caller writes its UI out top to
 * bottom every frame. That suits a program with a frame loop; it does not suit
 * a reactive toolkit, which keeps a component tree, diffs it, and emits
 * create/patch/append/remove against native widgets.
 *
 * This header is that widget layer. Nodes are integer handles, mutated by the
 * calls below; nothing is drawn until `vidya_tree_frame`, which paints the
 * whole tree at once. Interactions come back as a queue of events the caller
 * drains and routes to its own handlers — a callback cannot cross this
 * boundary, so identity does instead.
 *
 * The two halves share a window: open it with `vidya_open`, set the mode with
 * `vidya_set_mode`, and drive `vidya_tree_frame` in place of
 * `vidya_begin_frame` / `vidya_end_frame`. Do not mix them within one frame.
 *
 * Only the Rust/egui backend (`ffi/`) implements this header; the C/raylib
 * backend implements `vidya.h` alone. Every call stays on the thread that
 * called `vidya_open`, like the rest of the ABI.
 */
#ifndef VIDYA_TREE_H
#define VIDYA_TREE_H

#include "vidya.h"

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Node handles are positive; 0 is "no node" — a failed allocation, and the
 * `sibling` argument that means "first position".
 *
 * Tags are hiccup names without the colon:
 *
 *   containers  window box hbox vbox page card frame scroll
 *   widgets     label title title-2 dim-label button checkbutton entry
 *               separator spacer progress spinner status
 *
 * An unrecognized tag is kept and paints as a vertical box, so a caller ahead
 * of this backend still sees its children.
 *
 * Props are string-keyed, in the same names:
 *
 *   every node    sensitive (bool)
 *   box           orientation ("horizontal"|"vertical"), spacing, margin
 *   page          max-width
 *   scroll        orientation ("vertical"|"horizontal"|"both")
 *   text widgets  label, or text
 *   button        label, kind ("default"|"primary"|"destructive")
 *   checkbutton   label, active (bool)
 *   entry         text, placeholder, multiline (bool), rows
 *   spacer        size
 *   progress      value (0..1), label
 *   status        label, live (bool)
 */

/* The window node, created on first use. Mount everything under it. */
VIDYA_API int vidya_tree_root(void);

VIDYA_API int vidya_node_new(const char *tag);
VIDYA_API void vidya_node_free(int node);
VIDYA_API int vidya_node_exists(int node);

VIDYA_API void vidya_node_set_str(int node, const char *key, const char *value);
VIDYA_API void vidya_node_set_num(int node, const char *key, double value);
VIDYA_API void vidya_node_set_bool(int node, const char *key, int value);
VIDYA_API void vidya_node_clear_props(int node);

/*
 * Reads answer the empty string / 0 for a prop that is unset or of another
 * type. The returned pointer belongs to the library and is valid only until
 * the next string-returning call on this thread — copy it before the next one.
 */
VIDYA_API const char *vidya_node_get_str(int node, const char *key);
VIDYA_API double vidya_node_get_num(int node, const char *key);
VIDYA_API int vidya_node_get_bool(int node, const char *key);

/*
 * Reading the structure back. `vidya_node_tag` answers the canonical tag name
 * ("box" for both hbox and vbox), under the same borrowed-pointer rule as the
 * prop reads above; `vidya_node_child_at` answers 0 past the end.
 */
VIDYA_API const char *vidya_node_tag(int node);
VIDYA_API int vidya_node_child_count(int node);
VIDYA_API int vidya_node_child_at(int node, int index);

VIDYA_API int vidya_node_append(int parent, int child);
/* Unparents AND frees `child` with everything under it. */
VIDYA_API void vidya_node_remove(int parent, int child);
/* Moves `child` after `sibling`; `sibling` 0 means the first position. */
VIDYA_API int vidya_node_insert_after(int parent, int child, int sibling);
/* Puts `new_child` where `old_child` was, and frees `old_child`. */
VIDYA_API int vidya_node_replace(int parent, int old_child, int new_child);

/* Paint the whole tree as one frame. Inert with no window open. */
VIDYA_API void vidya_tree_frame(void);

/*
 * Drain interactions. `vidya_tree_poll_event` dequeues one and answers 1 while
 * there was one; the accessors describe whichever was dequeued last.
 *
 * Event names are glimmer's handler props without the `on-`:
 *
 *   click     a button was pressed              no payload
 *   toggled   a checkbutton changed             num is the new state
 *   change    an entry's text changed           text is the new text
 *   activate  Enter was pressed in an entry     no payload
 *
 * A widget does not own its value: `toggled` and `change` write the new state
 * back into the node's props as well, so a caller that ignores the event still
 * sees a working control, and the next prop write is what settles it.
 */
VIDYA_API int vidya_tree_poll_event(void);
VIDYA_API int vidya_tree_event_node(void);
VIDYA_API const char *vidya_tree_event_name(void);
VIDYA_API const char *vidya_tree_event_text(void);
VIDYA_API double vidya_tree_event_num(void);

#ifdef __cplusplus
}
#endif

#endif /* VIDYA_TREE_H */
