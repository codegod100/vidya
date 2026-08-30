#include <stddef.h>
#include <stdio.h>
#include <unistd.h>

#include <android/log.h>
#include <pthread.h>

#include "scheme.h"
#include "vidya.h"

extern const unsigned char _binary_jolt_boot_start[];
extern const unsigned char _binary_jolt_boot_end[];

#define REGISTER_VIDYA(name) Sregister_symbol(#name, (void *)&name)

static void register_vidya_api(void) {
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
}

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

int main(int argc, char **argv) {
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

  __android_log_print(ANDROID_LOG_INFO, "VidyaJolt",
                      "starting Jolt application");
  int status = Sscheme_start(argc, (const char **)argv);
  __android_log_print(ANDROID_LOG_ERROR, "VidyaJolt",
                      "Jolt application exited with status %d", status);
  Sscheme_deinit();
  return status;
}
