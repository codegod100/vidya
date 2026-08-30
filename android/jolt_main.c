/*
 * The Jolt half of the Android app: an embedded Chez runtime and the boot image
 * built from the Jolt sources, reached from `libvidya.so`'s `android_main`.
 *
 * This is not the NativeActivity. On Android the activity's library must be the
 * one holding android-activity's glue, which is `libvidya.so` — see
 * `ffi/src/android.rs`. That glue dlopens this library and calls
 * `vidya_jolt_main`, so the entry point below is an ordinary C function rather
 * than a `main`.
 *
 * Every Vidya symbol is registered with Chez explicitly. Jolt resolves a
 * `defcfn` through Chez's foreign-entry table, and on Android nothing populates
 * that table from the dynamic loader: the symbols live in a *different* shared
 * object (`libvidya.so`, a DT_NEEDED of this one), which is exactly the case
 * the loader will not answer for. Registering them by hand is what makes both
 * ABIs — the push/pop one and the retained tree one glimmer binds — reachable.
 */

#include <stddef.h>
#include <stdio.h>
#include <unistd.h>

#include <android/log.h>
#include <pthread.h>

#include "scheme.h"
#include "vidya.h"
#include "vidya_tree.h"

/* The boot image, linked in as a binary blob by llvm-objcopy. */
extern const unsigned char _binary_jolt_boot_start[];
extern const unsigned char _binary_jolt_boot_end[];

#define REGISTER_VIDYA(name) Sregister_symbol(#name, (void *)&name)

static void register_vidya_api(void) {
  /* vidya.h */
  REGISTER_VIDYA(vidya_open);
  REGISTER_VIDYA(vidya_close);
  REGISTER_VIDYA(vidya_should_close);
  REGISTER_VIDYA(vidya_set_target_fps);
  REGISTER_VIDYA(vidya_set_mode);
  REGISTER_VIDYA(vidya_get_mode);
  REGISTER_VIDYA(vidya_load_font);
  REGISTER_VIDYA(vidya_begin_frame);
  REGISTER_VIDYA(vidya_end_frame);
  REGISTER_VIDYA(vidya_page_begin);
  REGISTER_VIDYA(vidya_page_end);
  REGISTER_VIDYA(vidya_card_begin);
  REGISTER_VIDYA(vidya_card_end);
  REGISTER_VIDYA(vidya_gap);
  REGISTER_VIDYA(vidya_separator);
  REGISTER_VIDYA(vidya_title);
  REGISTER_VIDYA(vidya_title_2);
  REGISTER_VIDYA(vidya_body);
  REGISTER_VIDYA(vidya_dim_label);
  REGISTER_VIDYA(vidya_button);
  REGISTER_VIDYA(vidya_checkbox);
  REGISTER_VIDYA(vidya_checkbox_value);
  REGISTER_VIDYA(vidya_status);
  REGISTER_VIDYA(vidya_text_field);

  /* vidya_tree.h */
  REGISTER_VIDYA(vidya_tree_root);
  REGISTER_VIDYA(vidya_node_new);
  REGISTER_VIDYA(vidya_node_free);
  REGISTER_VIDYA(vidya_node_exists);
  REGISTER_VIDYA(vidya_node_set_str);
  REGISTER_VIDYA(vidya_node_set_num);
  REGISTER_VIDYA(vidya_node_set_bool);
  REGISTER_VIDYA(vidya_node_clear_props);
  REGISTER_VIDYA(vidya_node_get_str);
  REGISTER_VIDYA(vidya_node_get_num);
  REGISTER_VIDYA(vidya_node_get_bool);
  REGISTER_VIDYA(vidya_node_tag);
  REGISTER_VIDYA(vidya_node_child_count);
  REGISTER_VIDYA(vidya_node_child_at);
  REGISTER_VIDYA(vidya_node_append);
  REGISTER_VIDYA(vidya_node_remove);
  REGISTER_VIDYA(vidya_node_insert_after);
  REGISTER_VIDYA(vidya_node_replace);
  REGISTER_VIDYA(vidya_tree_frame);
  REGISTER_VIDYA(vidya_tree_poll_event);
  REGISTER_VIDYA(vidya_tree_event_node);
  REGISTER_VIDYA(vidya_tree_event_name);
  REGISTER_VIDYA(vidya_tree_event_text);
  REGISTER_VIDYA(vidya_tree_event_num);
}

/*
 * Chez writes to stdout/stderr, which on Android goes nowhere. Pump both into
 * logcat so a Scheme-level error is visible rather than a silent exit.
 */
static void *log_scheme_output(void *context) {
  int fd = *(int *)context;
  char buffer[1024];
  FILE *stream = fdopen(fd, "r");
  if (stream == NULL) return NULL;

  while (fgets(buffer, sizeof(buffer), stream) != NULL) {
    __android_log_write(ANDROID_LOG_ERROR, "VidyaJolt", buffer);
  }
  fclose(stream);
  return NULL;
}

int vidya_jolt_main(void) {
  static const char *argv[] = {"vidya", NULL};
  int output_pipe[2];
  pthread_t logger;

  if (pipe(output_pipe) == 0) {
    dup2(output_pipe[1], STDOUT_FILENO);
    dup2(output_pipe[1], STDERR_FILENO);
    close(output_pipe[1]);
    pthread_create(&logger, NULL, log_scheme_output, &output_pipe[0]);
    pthread_detach(logger);
  }

  __android_log_print(ANDROID_LOG_INFO, "VidyaJolt",
                      "initializing embedded Chez runtime");
  Sscheme_init(NULL);
  Sregister_boot_file_bytes(
      "jolt",
      (void *)_binary_jolt_boot_start,
      (iptr)(_binary_jolt_boot_end - _binary_jolt_boot_start));
  Sbuild_heap(NULL, NULL);
  register_vidya_api();

  __android_log_print(ANDROID_LOG_INFO, "VidyaJolt", "starting Jolt application");
  int status = Sscheme_start(1, argv);
  __android_log_print(ANDROID_LOG_ERROR, "VidyaJolt",
                      "Jolt application exited with status %d", status);
  Sscheme_deinit();
  return status;
}
