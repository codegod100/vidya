#define VIDYA_BUILD
#include "vidya.h"

#include "cimgui.h"
#include "raylib.h"
#include "rlImGui.h"

#include <stdio.h>

static VidyaMode mode = VIDYA_DARK;
static ImFont *font_body;
static ImFont *font_heading;
static int page_open;
static int card_depth;
static int card_serial;
static float requested_page_width;

static ImVec4_c rgba(unsigned int hex) {
    return (ImVec4_c){
        ((hex >> 24) & 255) / 255.0f,
        ((hex >> 16) & 255) / 255.0f,
        ((hex >> 8) & 255) / 255.0f,
        (hex & 255) / 255.0f
    };
}

static const char *first_font(const char *const *paths, int count) {
    for (int i = 0; i < count; i++) {
        if (FileExists(paths[i])) return paths[i];
    }
    return NULL;
}

static void load_fonts(void) {
    static const char *regular[] = {
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
    };
    static const char *bold[] = {
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-M.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
    };
    ImGuiIO *io = igGetIO_Nil();
    const char *regular_path =
        first_font(regular, (int)(sizeof(regular) / sizeof(regular[0])));
    const char *bold_path =
        first_font(bold, (int)(sizeof(bold) / sizeof(bold[0])));
    if (regular_path) {
        font_body = ImFontAtlas_AddFontFromFileTTF(
            io->Fonts, regular_path, 16, NULL, NULL);
    }
    if (bold_path) {
        font_heading = ImFontAtlas_AddFontFromFileTTF(
            io->Fonts, bold_path, 18, NULL, NULL);
    }
    if (!font_body) font_body = ImFontAtlas_AddFontDefault(io->Fonts, NULL);
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
}

int vidya_open(int width, int height, const char *title) {
    if (IsWindowReady()) return 0;
    SetConfigFlags(FLAG_WINDOW_RESIZABLE | FLAG_VSYNC_HINT |
                   FLAG_MSAA_4X_HINT);
    InitWindow(width, height, title ? title : "Vidya");
    if (!IsWindowReady()) return 0;
    SetExitKey(KEY_NULL);
    rlImGuiSetLoadFontsCallback(load_fonts);
    rlImGuiSetup(true);
    apply_style();
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
    (void)path;
    (void)atlas_size;
    /* Dear ImGui's atlas is uploaded during setup. Runtime replacement will be
     * added with atlas rebuild support; platform fonts are loaded at open. */
    return 0;
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
    EndDrawing();
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
    igPushFont(font_heading, 18 * scale);
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
        normal = rgba(0x3584e4ff);
        hover = rgba(0x4a93e7ff);
        active = rgba(0x1c71d8ff);
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
    return clicked;
}

int vidya_checkbox(const char *label, int *checked) {
    if (!checked) return 0;
    bool value = *checked != 0;
    bool changed = igCheckbox(label ? label : "", &value);
    *checked = value ? 1 : 0;
    return changed;
}

int vidya_checkbox_value(const char *label, int checked) {
    vidya_checkbox(label, &checked);
    return checked ? 1 : 0;
}

void vidya_status(const char *label, int live) {
    igPushStyleColor_Vec4(ImGuiCol_Text,
                          live ? rgba(0x2ec27eff) : rgba(0x9a9996ff));
    igBullet();
    igPopStyleColor(1);
    igSameLine(0, 6);
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
