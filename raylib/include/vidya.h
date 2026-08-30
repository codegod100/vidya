#ifndef VIDYA_H
#define VIDYA_H

#include <stddef.h>

#if defined(_WIN32)
#  if defined(VIDYA_BUILD)
#    define VIDYA_API __declspec(dllexport)
#  else
#    define VIDYA_API __declspec(dllimport)
#  endif
#elif defined(__GNUC__)
#  define VIDYA_API __attribute__((visibility("default")))
#else
#  define VIDYA_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef enum VidyaMode {
    VIDYA_DARK = 0,
    VIDYA_LIGHT = 1
} VidyaMode;

typedef enum VidyaButtonKind {
    VIDYA_BUTTON_DEFAULT = 0,
    VIDYA_BUTTON_PRIMARY = 1,
    VIDYA_BUTTON_DESTRUCTIVE = 2
} VidyaButtonKind;

/* Window and frame lifecycle. There is one UI context per process for now,
 * matching raylib's window model. All calls must remain on the window thread. */
VIDYA_API int vidya_open(int width, int height, const char *title);
VIDYA_API void vidya_close(void);
VIDYA_API int vidya_should_close(void);
VIDYA_API void vidya_set_target_fps(int fps);
VIDYA_API void vidya_set_mode(int mode);
VIDYA_API int vidya_get_mode(void);
/* Replace the UI font. Returns 1 on success. A 32px atlas is a good default;
 * controls scale it down to the semantic type sizes while retaining quality. */
VIDYA_API int vidya_load_font(const char *path, int atlas_size);
VIDYA_API void vidya_begin_frame(void);
VIDYA_API void vidya_end_frame(void);

/* A frame is a vertical immediate-mode page. Containers save and restore its
 * horizontal bounds; every leaf advances the vertical cursor. */
VIDYA_API void vidya_page_begin(float max_width);
VIDYA_API void vidya_page_end(void);
VIDYA_API void vidya_card_begin(void);
VIDYA_API void vidya_card_end(void);
VIDYA_API void vidya_gap(float pixels);
VIDYA_API void vidya_separator(void);

/* Semantic text roles. */
VIDYA_API void vidya_title(const char *text);
VIDYA_API void vidya_title_2(const char *text);
VIDYA_API void vidya_body(const char *text);
VIDYA_API void vidya_dim_label(const char *text);

/* Controls return 1 only on activation/change during the current frame. */
VIDYA_API int vidya_button(const char *label, int kind);
VIDYA_API int vidya_checkbox(const char *label, int *checked);
/* FFI-friendly checkbox variant: returns the current value after handling input. */
VIDYA_API int vidya_checkbox_value(const char *label, int checked);
VIDYA_API void vidya_status(const char *label, int live);
VIDYA_API int vidya_text_field(char *text, size_t capacity, const char *placeholder);

#ifdef __cplusplus
}
#endif

#endif
