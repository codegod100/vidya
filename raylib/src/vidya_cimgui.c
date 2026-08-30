#define VIDYA_BUILD
#include "vidya.h"

#include "cimgui.h"
#include "raylib.h"
#include "rlImGui.h"

#include <stdio.h>
#include <string.h>
#if defined(__ANDROID__)
#include <android/log.h>
#endif

static VidyaMode mode = VIDYA_DARK;
static ImFont *font_body;
static ImFont *font_heading;
static int page_open;
static int card_depth;
static int card_serial;
static float requested_page_width;

/* Density multiplier for every size in the theme. Override at build time with
 * -DVIDYA_UI_SCALE=<float> when the default is wrong for a device. */
#ifndef VIDYA_UI_SCALE
#  if defined(__ANDROID__)
#    define VIDYA_UI_SCALE 1.35f
#  else
#    define VIDYA_UI_SCALE 1.0f
#  endif
#endif
static const float ui_scale = VIDYA_UI_SCALE;

/* Drag-to-scroll gesture state. A page taller than the screen scrolls with the
 * wheel on its own; touch screens have no wheel, so a press-and-drag anywhere
 * in the page pans it, and a flick keeps going. Once a drag has travelled far
 * enough to count as a scroll, clicks are swallowed until the finger lifts —
 * otherwise every pan would also press whatever it started on. */
static float scroll_velocity;
static float drag_travel;
static int scroll_gesture;

#define VIDYA_SCROLL_SLOP 8.0f
#define VIDYA_SCROLL_DECAY 0.92f

/* One accent ramp for every widget that paints itself blue: the primary
 * button, a checked checkbox, selected text, the keyboard cursor. */
#define VIDYA_ACCENT 0x3584e4ffu
#define VIDYA_ACCENT_HOVER 0x4a93e7ffu
#define VIDYA_ACCENT_ACTIVE 0x1c71d8ffu

static ImVec4_c rgba(unsigned int hex) {
    return (ImVec4_c){
        ((hex >> 24) & 255) / 255.0f,
        ((hex >> 16) & 255) / 255.0f,
        ((hex >> 8) & 255) / 255.0f,
        (hex & 255) / 255.0f
    };
}

/* Fonts.
 *
 * Dear ImGui 1.92 rasterizes glyphs on demand, so a face is registered once at
 * a reference size and igPushFont(font, size) may then ask for any other size.
 * Three things decide how the result looks:
 *
 *   - the rasterizer. FreeType with light hinting is what desktop toolkits use
 *     for interface text; builds without it fall back to Dear ImGui's bundled
 *     stb_truetype, which is softer at UI sizes.
 *   - the density. rlImGui reports the window's DPI scale through
 *     io.DisplayFramebufferScale and Dear ImGui bakes glyphs at that density,
 *     so FLAG_WINDOW_HIGHDPI in vidya_open is what keeps HiDPI text crisp.
 *   - the face. Dear ImGui's built-in font is a fallback, not an interface
 *     font, so the search below covers the desktops and Android rather than
 *     only Debian's font paths.
 */

#define VIDYA_BODY_SIZE 16.0f
#define VIDYA_HEADING_SIZE 18.0f

#if defined(VIDYA_EMBED_SYMBOL_FONT)
extern const unsigned char vidya_symbol_font[];
extern const unsigned int vidya_symbol_font_size;
#endif

typedef struct {
    const char *regular;
    const char *bold; /* NULL when the family installs a single weight. */
} VidyaFontFamily;

/* Both weights come from one family so headings never switch typeface. Ubuntu
 * stays first because the egui implementation of Vidya ships it and the two
 * should look alike; the rest widen the search past Debian's layout. */
static const VidyaFontFamily ui_families[] = {
    {"/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
     "/usr/share/fonts/truetype/ubuntu/Ubuntu-M.ttf"},
    /* GNOME's interface fonts. Both install one variable file, so their bold
     * is a named instance of the regular rather than a second path. */
    {"/usr/share/fonts/Adwaita/AdwaitaSans-Regular.ttf", NULL},
    {"/usr/share/fonts/cantarell/Cantarell-VF.otf", NULL},
    {"/usr/share/fonts/abattis-cantarell/Cantarell-VF.otf", NULL},
    {"/usr/share/fonts/truetype/cantarell/Cantarell-VF.otf", NULL},
    /* Distribution defaults. */
    {"/usr/share/fonts/liberation-fonts/LiberationSans-Regular.ttf",
     "/usr/share/fonts/liberation-fonts/LiberationSans-Bold.ttf"},
    {"/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
     "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf"},
    {"/usr/share/fonts/dejavu/DejaVuSans.ttf",
     "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf"},
    {"/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
     "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"},
    {"/usr/share/fonts/TTF/DejaVuSans.ttf",
     "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf"},
    {"/home/linuxbrew/.linuxbrew/share/fonts/dejavu/DejaVuSans.ttf",
     "/home/linuxbrew/.linuxbrew/share/fonts/dejavu/DejaVuSans-Bold.ttf"},
    /* macOS. */
    {"/System/Library/Fonts/SFNS.ttf", NULL},
    {"/System/Library/Fonts/Helvetica.ttc", NULL},
    {"/opt/homebrew/share/fonts/dejavu/DejaVuSans.ttf",
     "/opt/homebrew/share/fonts/dejavu/DejaVuSans-Bold.ttf"},
    /* Windows. */
    {"C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/segoeuib.ttf"},
    {"C:/Windows/Fonts/arial.ttf", "C:/Windows/Fonts/arialbd.ttf"},
    /* Android. Without these the NativeActivity fell back to Dear ImGui's
     * built-in font. */
    {"/system/fonts/Roboto-Regular.ttf", "/system/fonts/Roboto-Medium.ttf"},
    {"/system/fonts/NotoSans-Regular.ttf", "/system/fonts/NotoSans-Bold.ttf"},
    {"/system/fonts/DroidSans.ttf", "/system/fonts/DroidSans-Bold.ttf"}
};

static ImFontConfig *face_config(int embolden) {
    ImFontConfig *cfg = ImFontConfig_ImFontConfig();
#if defined(IMGUI_ENABLE_FREETYPE)
    /* Snap to the pixel grid vertically only. Full hinting sharpens stems but
     * distorts letterforms and spacing at 16px, so desktop rasterizers settled
     * on the lighter variant for interface text. */
    cfg->FontLoaderFlags = ImGuiFreeTypeLoaderFlags_LightHinting;
    if (embolden) cfg->FontLoaderFlags |= ImGuiFreeTypeLoaderFlags_Bold;
#else
    (void)embolden;
#endif
    return cfg;
}

/* Merge the bundled DejaVu subset behind `dst` so arrows, bullets, curly
 * quotes, box drawing and math-in-prose still draw when the interface font has
 * no glyph for them. Mirrors vidya::fonts in the egui implementation. */
static void merge_symbol_face(ImFontAtlas *atlas, ImFont *dst, float size,
                              int embolden) {
#if defined(VIDYA_EMBED_SYMBOL_FONT)
    ImFontConfig *cfg = face_config(embolden);
    cfg->MergeMode = true;
    /* The array has static storage; the atlas must not free it. */
    cfg->FontDataOwnedByAtlas = false;
    cfg->DstFont = dst;
    ImFontAtlas_AddFontFromMemoryTTF(atlas, (void *)vidya_symbol_font,
                                     (int)vidya_symbol_font_size, size, cfg,
                                     NULL);
    ImFontConfig_destroy(cfg);
#else
    (void)atlas;
    (void)dst;
    (void)size;
    (void)embolden;
#endif
}

static ImFont *add_face(ImFontAtlas *atlas, const char *path, float size,
                        int embolden) {
    ImFontConfig *cfg = face_config(embolden);
    ImFont *font = ImFontAtlas_AddFontFromFileTTF(atlas, path, size, cfg, NULL);
    ImFontConfig_destroy(cfg);
    if (font) merge_symbol_face(atlas, font, size, embolden);
    return font;
}

#if defined(IMGUI_ENABLE_FREETYPE)
static unsigned int be16(const unsigned char *p) {
    return ((unsigned int)p[0] << 8) | p[1];
}

static unsigned int be32(const unsigned char *p) {
    return ((unsigned int)p[0] << 24) | ((unsigned int)p[1] << 16) |
           ((unsigned int)p[2] << 8) | p[3];
}

/* One-based index of the bold named instance of a variable font, or 0 when the
 * file has no weight axis or no instance near bold. GNOME's interface fonts
 * (Adwaita Sans, Cantarell) ship every weight in one variable file, so without
 * this a heading would have to be a synthetic embolden, which barely reads as
 * bold at UI sizes. Layout is `fvar` from the OpenType specification. */
static unsigned int bold_instance(const unsigned char *data, unsigned int size) {
    if (size < 12) return 0;
    unsigned int table_count = be16(data + 4);
    unsigned int fvar = 0;
    for (unsigned int i = 0; i < table_count; i++) {
        unsigned int entry = 12 + 16 * i;
        if (entry + 16 > size) return 0;
        if (memcmp(data + entry, "fvar", 4) == 0) {
            fvar = be32(data + entry + 8);
            break;
        }
    }
    /* Bound every array up front, so the reads below need no further checks.
     * Counts and sizes are 16 bit, so their products cannot overflow. */
    if (fvar == 0 || fvar > size || size - fvar < 16) return 0;

    unsigned int axes = fvar + be16(data + fvar + 4);
    unsigned int axis_count = be16(data + fvar + 8);
    unsigned int axis_size = be16(data + fvar + 10);
    unsigned int instance_count = be16(data + fvar + 12);
    unsigned int instance_size = be16(data + fvar + 14);
    if (axis_count == 0 || axis_size < 20 ||
        instance_size < 4 + 4 * axis_count) return 0;
    if (axes > size || size - axes < axis_count * axis_size) return 0;
    unsigned int instances = axes + axis_count * axis_size;
    if (size - instances < instance_count * instance_size) return 0;

    unsigned int weight_axis = axis_count;
    for (unsigned int i = 0; i < axis_count; i++) {
        if (memcmp(data + axes + i * axis_size, "wght", 4) == 0) {
            weight_axis = i;
            break;
        }
    }
    if (weight_axis == axis_count) return 0;

    unsigned int closest = 0;
    unsigned int closest_distance = 0;
    for (unsigned int i = 0; i < instance_count; i++) {
        unsigned int instance = instances + i * instance_size;
        /* subfamilyNameID, flags, then one 16.16 coordinate per axis. */
        unsigned int weight =
            be32(data + instance + 4 + 4 * weight_axis) >> 16;
        unsigned int distance = weight > 700 ? weight - 700 : 700 - weight;
        if (closest == 0 || distance < closest_distance) {
            closest = i + 1; /* FreeType numbers named instances from one. */
            closest_distance = distance;
        }
    }
    /* A family whose heaviest instance is semibold or lighter has no bold. */
    return closest_distance <= 100 ? closest : 0;
}
#endif

/* Register the bold named instance of a variable font. FreeType selects one
 * through the high half of the face index, which is what FontNo becomes. */
static ImFont *add_variable_bold(ImFontAtlas *atlas, const char *path,
                                 float size) {
#if defined(IMGUI_ENABLE_FREETYPE)
    int bytes = 0;
    unsigned char *data = LoadFileData(path, &bytes);
    if (!data || bytes <= 0) {
        if (data) UnloadFileData(data);
        return NULL;
    }
    unsigned int instance = bold_instance(data, (unsigned int)bytes);
    UnloadFileData(data);
    if (instance == 0) return NULL;

    ImFontConfig *cfg = face_config(0);
    cfg->FontNo = instance << 16;
    ImFont *font = ImFontAtlas_AddFontFromFileTTF(atlas, path, size, cfg, NULL);
    ImFontConfig_destroy(cfg);
    if (font) merge_symbol_face(atlas, font, size, 0);
    return font;
#else
    (void)atlas;
    (void)path;
    (void)size;
    return NULL;
#endif
}

/* Heading weight, best source first: a bold file, the bold instance of a
 * variable file, then a synthetic embolden of the regular face. */
static ImFont *add_bold_face(ImFontAtlas *atlas, const char *regular,
                             const char *bold, float size) {
    ImFont *font = NULL;
    if (bold && FileExists(bold)) font = add_face(atlas, bold, size, 0);
    if (!font) font = add_variable_bold(atlas, regular, size);
    if (!font) font = add_face(atlas, regular, size, 1);
    return font;
}

static const VidyaFontFamily *first_family(void) {
    int count = (int)(sizeof(ui_families) / sizeof(ui_families[0]));
    const VidyaFontFamily *fallback = NULL;
    for (int i = 0; i < count; i++) {
        const VidyaFontFamily *family = &ui_families[i];
        if (!FileExists(family->regular)) continue;
        if (!fallback) fallback = family;
#if !defined(IMGUI_ENABLE_FREETYPE)
        /* stb_truetype offers neither variable instances nor a synthetic bold,
         * so a single-weight family there would flatten every heading. */
        if (!family->bold || !FileExists(family->bold)) continue;
#endif
        return family;
    }
    return fallback;
}

static void load_fonts(void) {
    ImGuiIO *io = igGetIO_Nil();
    const float body_size = VIDYA_BODY_SIZE * ui_scale;
    const float heading_size = VIDYA_HEADING_SIZE * ui_scale;
    const VidyaFontFamily *family = first_family();

    if (family) {
        font_body = add_face(io->Fonts, family->regular, body_size, 0);
        /* A regular that would not load makes its bold moot as well. */
        if (font_body) {
            font_heading = add_bold_face(io->Fonts, family->regular,
                                         family->bold, heading_size);
        }
    }
    if (!font_body) {
        /* The vector default rather than the 13px bitmap one: headings and the
         * Android scale factor both need a face that scales. */
        ImFontConfig *cfg = ImFontConfig_ImFontConfig();
        cfg->SizePixels = body_size;
        font_body = ImFontAtlas_AddFontDefaultVector(io->Fonts, cfg);
        ImFontConfig_destroy(cfg);
    }
    if (!font_heading) font_heading = font_body;
    io->FontDefault = font_body;
}

static void apply_style(void) {
    ImGuiStyle *s = igGetStyle();
    igStyleColorsDark(s);
    s->WindowPadding = (ImVec2_c){16, 16};
    s->FramePadding = (ImVec2_c){12, 8};
    s->CellPadding = (ImVec2_c){12, 8};
    s->ItemSpacing = (ImVec2_c){8, 8};
    s->ItemInnerSpacing = (ImVec2_c){6, 6};
    s->IndentSpacing = 18;
    s->ScrollbarSize = 12;
    s->WindowRounding = 0;
    s->ChildRounding = 10;
    s->FrameRounding = 8;
    s->PopupRounding = 10;
    s->GrabRounding = 8;
    s->WindowBorderSize = 0;
    s->ChildBorderSize = 1;
    s->FrameBorderSize = 1;

    /* rlImGui uploads the atlas through LoadTextureFromImage, and raylib gives
     * a single-mipmap texture GL_NEAREST on both filters. The anti-aliasing
     * ramp Dear ImGui bakes into that atlas for stroked lines therefore samples
     * as a hard step. Geometry-based line anti-aliasing costs a few vertices
     * per border and is correct whatever the sampler does. */
    s->AntiAliasedLinesUseTex = false;
    /* Largest error, in pixels, tolerated when tessellating a corner arc.
     * Halving the default is sub-pixel at the 8-10px radii used here, but it
     * costs a handful of vertices and keeps larger radii honest. */
    s->CircleTessellationMaxError = 0.10f;

    ImVec4_c *c = s->Colors;
    if (mode == VIDYA_LIGHT) {
        c[ImGuiCol_Text] = rgba(0x241f31ff);
        c[ImGuiCol_TextDisabled] = rgba(0x5e5c64ff);
        c[ImGuiCol_WindowBg] = rgba(0xfafafaff);
        c[ImGuiCol_ChildBg] = rgba(0xffffffff);
        c[ImGuiCol_Border] = rgba(0xe0e0e0ff);
        c[ImGuiCol_FrameBg] = rgba(0xe0e0e0ff);
        c[ImGuiCol_FrameBgHovered] = rgba(0xd0d0d0ff);
        c[ImGuiCol_FrameBgActive] = rgba(0xc0c0c0ff);
    } else {
        c[ImGuiCol_Text] = rgba(0xffffffff);
        c[ImGuiCol_TextDisabled] = rgba(0x9a9996ff);
        c[ImGuiCol_WindowBg] = rgba(0x242424ff);
        c[ImGuiCol_ChildBg] = rgba(0x303030ff);
        c[ImGuiCol_Border] = rgba(0x3d3846ff);
        c[ImGuiCol_FrameBg] = rgba(0x3d3d3dff);
        c[ImGuiCol_FrameBgHovered] = rgba(0x4a4a4aff);
        c[ImGuiCol_FrameBgActive] = rgba(0x353535ff);
    }
    c[ImGuiCol_CheckMark] = rgba(0xffffffff);
    c[ImGuiCol_Button] = c[ImGuiCol_FrameBg];
    c[ImGuiCol_ButtonHovered] = c[ImGuiCol_FrameBgHovered];
    c[ImGuiCol_ButtonActive] = c[ImGuiCol_FrameBgActive];
    c[ImGuiCol_Header] = rgba(0x3584e459);
    c[ImGuiCol_HeaderHovered] = rgba(0x4a93e777);
    c[ImGuiCol_HeaderActive] = rgba(0x1c71d899);
    c[ImGuiCol_ScrollbarBg] = (ImVec4_c){0, 0, 0, 0};
    c[ImGuiCol_ScrollbarGrab] = rgba(0x5e5c6488);

    /* igStyleColorsDark filled every entry above, so anything left unassigned
     * keeps Dear ImGui's stock blue-grey and reads as a different theme the
     * moment a widget enters that state. These are the ones this widget set can
     * actually reach; a checked checkbox in particular paints itself with
     * CheckboxSelectedBg rather than FrameBg. */
    c[ImGuiCol_CheckboxSelectedBg] = rgba(VIDYA_ACCENT);
    c[ImGuiCol_NavCursor] = rgba(VIDYA_ACCENT);
    c[ImGuiCol_TextLink] = rgba(VIDYA_ACCENT);
    c[ImGuiCol_TextSelectedBg] = rgba(0x3584e455);
    c[ImGuiCol_InputTextCursor] = c[ImGuiCol_Text];
    c[ImGuiCol_PopupBg] = c[ImGuiCol_ChildBg];
    c[ImGuiCol_BorderShadow] = (ImVec4_c){0, 0, 0, 0};
    /* A separator sits inside a card, so it should match that card's border
     * rather than being brighter than the edge enclosing it. */
    c[ImGuiCol_Separator] = c[ImGuiCol_Border];
    c[ImGuiCol_SeparatorHovered] = c[ImGuiCol_Border];
    c[ImGuiCol_SeparatorActive] = c[ImGuiCol_Border];
    c[ImGuiCol_ScrollbarGrabHovered] = rgba(0x5e5c64bb);
    c[ImGuiCol_ScrollbarGrabActive] = rgba(0x5e5c64dd);
}

int vidya_open(int width, int height, const char *title) {
    if (IsWindowReady()) return 0;
#if defined(__ANDROID__)
    /* InitWindow's dimensions are the logical render surface on Android, not a
     * desktop window request. Keep the physical device's portrait aspect ratio
     * so raylib does not letterbox the ImGui framebuffer into a short strip. */
    width = 720;
    height = 1600;
#endif
    unsigned int flags =
        FLAG_WINDOW_RESIZABLE | FLAG_VSYNC_HINT | FLAG_MSAA_4X_HINT;
#if !defined(__ANDROID__)
    /* Give the window a framebuffer at the monitor's real pixel density.
     * rlImGui forwards that scale as io.DisplayFramebufferScale, which is what
     * Dear ImGui bakes glyphs at, so HiDPI text is rasterized rather than
     * magnified. Layout stays in logical units either way. */
    flags |= FLAG_WINDOW_HIGHDPI;
#endif
    SetConfigFlags(flags);
    InitWindow(width, height, title ? title : "Vidya");
    if (!IsWindowReady()) return 0;
    SetExitKey(KEY_NULL);
    rlImGuiSetLoadFontsCallback(load_fonts);
    rlImGuiSetup(true);
    apply_style();
    if (ui_scale != 1.0f) ImGuiStyle_ScaleAllSizes(igGetStyle(), ui_scale);
    return 1;
}

void vidya_close(void) {
    if (!IsWindowReady()) return;
    rlImGuiShutdown();
    CloseWindow();
}

int vidya_should_close(void) { return WindowShouldClose(); }
void vidya_set_target_fps(int fps) { SetTargetFPS(fps); }
void vidya_set_mode(int value) {
    mode = value == VIDYA_LIGHT ? VIDYA_LIGHT : VIDYA_DARK;
    if (IsWindowReady()) apply_style();
}
int vidya_get_mode(void) { return mode; }

int vidya_load_font(const char *path, int atlas_size) {
    /* atlas_size is a rasterization hint for the direct backend's fixed atlas.
     * Dear ImGui bakes each size on demand, so it is ignored here and the
     * semantic type scale is preserved. */
    (void)atlas_size;
    if (!IsWindowReady() || !path || !FileExists(path)) return 0;
    ImGuiIO *io = igGetIO_Nil();
    ImFont *body =
        add_face(io->Fonts, path, VIDYA_BODY_SIZE * ui_scale, 0);
    if (!body) return 0;
    /* Headings come from this file too rather than keeping a bold belonging to
     * the family it replaced. */
    ImFont *heading =
        add_bold_face(io->Fonts, path, NULL, VIDYA_HEADING_SIZE * ui_scale);
    font_body = body;
    font_heading = heading ? heading : body;
    io->FontDefault = font_body;
    return 1;
}

void vidya_begin_frame(void) {
    BeginDrawing();
    ClearBackground(mode == VIDYA_LIGHT ? (Color){250,250,250,255}
                                       : (Color){36,36,36,255});
    rlImGuiBegin();
    page_open = 0;
    card_depth = 0;
    card_serial = 0;
}

void vidya_end_frame(void) {
    while (card_depth-- > 0) igEndChild();
    if (page_open) igEnd();
    rlImGuiEnd();
#if defined(__ANDROID__)
    static int logged_frame;
    if (!logged_frame) {
        ImGuiIO *io = igGetIO_Nil();
        ImDrawData *draw = igGetDrawData();
        __android_log_print(
            ANDROID_LOG_INFO, "vidya",
            "screen=%dx%d display=%.0fx%.0f fb=%.1fx%.1f lists=%d vertices=%d",
            GetScreenWidth(), GetScreenHeight(),
            io->DisplaySize.x, io->DisplaySize.y,
            io->DisplayFramebufferScale.x, io->DisplayFramebufferScale.y,
            draw ? draw->CmdLists.Size : -1,
            draw ? draw->TotalVtxCount : -1);
        logged_frame = 1;
    }
#endif
    EndDrawing();
}

/* Pan the current window from pointer drags, then coast. Call just inside
 * igBegin, before any of the page's own widgets. */
static void drag_scroll(void) {
    ImGuiIO *io = igGetIO_Nil();
    if (igGetScrollMaxY() <= 0.0f) {
        scroll_velocity = 0.0f;
        drag_travel = 0.0f;
        scroll_gesture = 0;
        return;
    }

    if (io->MouseDown[0]) {
        int inside = igIsWindowHovered(ImGuiHoveredFlags_ChildWindows |
                                       ImGuiHoveredFlags_AllowWhenBlockedByActiveItem);
        float dy = io->MouseDelta.y;
        if (inside) {
            drag_travel += dy < 0 ? -dy : dy;
            if (drag_travel > VIDYA_SCROLL_SLOP * ui_scale) scroll_gesture = 1;
            if (scroll_gesture && dy != 0.0f) {
                igSetScrollY_Float(igGetScrollY() - dy);
                scroll_velocity = dy;
            }
        }
        return;
    }

    drag_travel = 0.0f;
    /* Clear a frame late: Dear ImGui reports a button click on release, and
     * that release is the last frame of the gesture we are swallowing. */
    if (!io->MouseReleased[0]) scroll_gesture = 0;

    if (scroll_velocity != 0.0f) {
        igSetScrollY_Float(igGetScrollY() - scroll_velocity);
        scroll_velocity *= VIDYA_SCROLL_DECAY;
        if (scroll_velocity < 0.5f && scroll_velocity > -0.5f) scroll_velocity = 0.0f;
    }
}

void vidya_page_begin(float max_width) {
    requested_page_width = max_width;
    igSetNextWindowPos((ImVec2_c){0, 0}, ImGuiCond_Always,
                       (ImVec2_c){0, 0});
    igSetNextWindowSize((ImVec2_c){(float)GetScreenWidth(),
                                   (float)GetScreenHeight()},
                        ImGuiCond_Always);
    ImGuiWindowFlags flags =
        ImGuiWindowFlags_NoDecoration | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoSavedSettings |
        ImGuiWindowFlags_NoBringToFrontOnFocus;
    igBegin("##vidya-root", NULL, flags);
    drag_scroll();
    if (max_width > 0 && max_width < GetScreenWidth() - 32) {
        igSetCursorPosX(((float)GetScreenWidth() - max_width) * 0.5f);
        igPushItemWidth(max_width);
    }
    page_open = 1;
}

void vidya_page_end(void) {
    if (!page_open) return;
    if (requested_page_width > 0) igPopItemWidth();
    igEnd();
    page_open = 0;
}

void vidya_card_begin(void) {
    char id[32];
    snprintf(id, sizeof(id), "##vidya-card-%d", card_serial++);
    card_depth++;
    igBeginChild_Str(id, (ImVec2_c){-1, 0},
                     ImGuiChildFlags_Borders |
                     ImGuiChildFlags_AutoResizeY,
                     ImGuiWindowFlags_None);
}

void vidya_card_end(void) {
    if (card_depth <= 0) return;
    igEndChild();
    card_depth--;
}

void vidya_gap(float pixels) { igDummy((ImVec2_c){0, pixels}); }
void vidya_separator(void) { igSeparator(); }

static void heading(const char *text, float scale) {
    igPushFont(font_heading, 18 * scale * ui_scale);
    igTextUnformatted(text ? text : "", NULL);
    igPopFont();
}

void vidya_title(const char *text) { heading(text, 1.15f); }
void vidya_title_2(const char *text) { heading(text, 1.0f); }
void vidya_body(const char *text) {
    igPushTextWrapPos(0);
    igTextUnformatted(text ? text : "", NULL);
    igPopTextWrapPos();
}
void vidya_dim_label(const char *text) {
    igPushStyleColor_Vec4(ImGuiCol_Text, igGetStyle()->Colors[ImGuiCol_TextDisabled]);
    igTextUnformatted(text ? text : "", NULL);
    igPopStyleColor(1);
}

int vidya_button(const char *label, int kind) {
    ImVec4_c normal, hover, active, text;
    int pushed = 0;
    if (kind == VIDYA_BUTTON_PRIMARY) {
        normal = rgba(VIDYA_ACCENT);
        hover = rgba(VIDYA_ACCENT_HOVER);
        active = rgba(VIDYA_ACCENT_ACTIVE);
        text = rgba(0xffffffff);
        pushed = 1;
    } else if (kind == VIDYA_BUTTON_DESTRUCTIVE) {
        normal = rgba(0xc01c28ff);
        hover = rgba(0xd52b38ff);
        active = rgba(0xa51a24ff);
        text = rgba(0xffffffff);
        pushed = 1;
    }
    if (pushed) {
        igPushStyleColor_Vec4(ImGuiCol_Button, normal);
        igPushStyleColor_Vec4(ImGuiCol_ButtonHovered, hover);
        igPushStyleColor_Vec4(ImGuiCol_ButtonActive, active);
        igPushStyleColor_Vec4(ImGuiCol_Text, text);
    }
    int clicked = igButton(label ? label : "", (ImVec2_c){0, 0});
    if (pushed) igPopStyleColor(4);
    return scroll_gesture ? 0 : clicked;
}

int vidya_checkbox(const char *label, int *checked) {
    if (!checked) return 0;
    bool value = *checked != 0;
    /* Dear ImGui sizes the check square as FontSize + FramePadding.y * 2. The
     * padding that gives buttons and entries their height makes that 32px
     * beside a 16px label, about twice the proportion the HIG draws. Pad the
     * square on its own; the row keeps the shared vertical rhythm because
     * ItemSpacing is untouched. */
    ImGuiStyle *s = igGetStyle();
    igPushStyleVar_Vec2(ImGuiStyleVar_FramePadding,
                        (ImVec2_c){s->FramePadding.x, 3 * ui_scale});
    bool changed = igCheckbox(label ? label : "", &value);
    igPopStyleVar(1);
    *checked = value ? 1 : 0;
    return changed;
}

int vidya_checkbox_value(const char *label, int checked) {
    int toggled = checked;
    vidya_checkbox(label, &toggled);
    if (scroll_gesture) return checked ? 1 : 0;
    return toggled ? 1 : 0;
}

void vidya_status(const char *label, int live) {
    /* igBullet draws a dot of FontSize * 0.2 and indents the row like a list
     * item, which puts a speck hard against the label and out of line with the
     * controls above it. Draw the dot at a size that reads as a status light,
     * centred on the text line, starting at the row's own left edge. */
    const float diameter = 8 * ui_scale;
    const float line = igGetTextLineHeight();
    ImVec2_c origin = igGetCursorScreenPos();
    ImVec2_c center = {origin.x + diameter * 0.5f, origin.y + line * 0.5f};
    ImDrawList_AddCircleFilled(
        igGetWindowDrawList(), center, diameter * 0.5f,
        igColorConvertFloat4ToU32(live ? rgba(0x2ec27eff) : rgba(0x9a9996ff)),
        0);
    igDummy((ImVec2_c){diameter, line});
    igSameLine(0, 8 * ui_scale);
    igTextUnformatted(label ? label : "", NULL);
}

int vidya_text_field(char *text, size_t capacity, const char *placeholder) {
    if (!text || capacity == 0) return 0;
    igSetNextItemWidth(-1);
    return igInputTextWithHint("##vidya-field",
                               placeholder ? placeholder : "",
                               text, capacity, ImGuiInputTextFlags_None,
                               NULL, NULL);
}
