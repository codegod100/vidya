#define VIDYA_BUILD
#include "vidya.h"

#include "raylib.h"

#include <math.h>
#include <string.h>

typedef struct Palette {
    Color window_bg, view_bg, card_bg, popover_bg;
    Color accent, accent_fg, accent_hover, accent_active;
    Color destructive, success, warning;
    Color text, text_secondary, text_disabled;
    Color border, border_soft;
    Color button_bg, button_hover, button_active, button_fg;
} Palette;

typedef struct Layout {
    float x, y, width;
} Layout;

typedef struct Card {
    Layout parent;
    float top;
    Color parent_fill;
} Card;

static VidyaMode mode = VIDYA_DARK;
static Layout layout;
static Card cards[16];
static int card_depth;
static int active_field;
static int next_id;
static Font ui_font;
static int owns_ui_font;
static Font ui_font_bold;
static int owns_ui_font_bold;

static const float XS = 4, SM = 6, MD = 12, LG = 18, PAGE = 16;
static const float CONTROL_H = 34, RADIUS_SM = 6, RADIUS_MD = 9;

static Color rgb(unsigned char r, unsigned char g, unsigned char b) {
    return (Color){r, g, b, 255};
}

static Palette palette(void) {
    if (mode == VIDYA_LIGHT) {
        return (Palette){
            rgb(250,250,250), rgb(255,255,255), rgb(255,255,255), rgb(255,255,255),
            rgb(53,132,228), WHITE, rgb(74,147,231), rgb(28,113,216),
            rgb(192,28,40), rgb(38,162,105), rgb(229,165,10),
            rgb(36,31,49), rgb(94,92,100), rgb(154,153,150),
            rgb(205,205,205), rgb(224,224,224),
            rgb(224,224,224), rgb(208,208,208), rgb(192,192,192), rgb(36,31,49)
        };
    }
    return (Palette){
        rgb(36,36,36), rgb(30,30,30), rgb(48,48,48), rgb(56,56,56),
        rgb(53,132,228), WHITE, rgb(74,147,231), rgb(28,113,216),
        rgb(192,28,40), rgb(46,194,126), rgb(229,165,10),
        WHITE, rgb(154,153,150), rgb(94,92,100),
        rgb(94,92,100), rgb(61,56,70),
        rgb(61,61,61), rgb(74,74,74), rgb(53,53,53), WHITE
    };
}

static int load_ui_font(const char *path, int atlas_size) {
    if (!path || !FileExists(path)) return 0;
    Font loaded = LoadFontEx(path, atlas_size > 0 ? atlas_size : 32, NULL, 0);
    /* On failure raylib returns its valid default bitmap font, so validity
     * alone is insufficient to detect an unsupported file (e.g. some variable
     * OpenType fonts). */
    if (!IsFontValid(loaded) ||
        loaded.texture.id == GetFontDefault().texture.id) return 0;
    SetTextureFilter(loaded.texture, TEXTURE_FILTER_BILINEAR);
    if (owns_ui_font) UnloadFont(ui_font);
    ui_font = loaded;
    owns_ui_font = 1;
    return 1;
}

static void load_platform_font(void) {
    static const char *candidates[] = {
        /* GNOME/Linux */
        "/usr/share/fonts/cantarell/Cantarell-VF.otf",
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-L.ttf",
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        /* Homebrew Linux and macOS */
        "/home/linuxbrew/.linuxbrew/share/fonts/dejavu/DejaVuSans.ttf",
        "/opt/homebrew/share/fonts/dejavu/DejaVuSans.ttf",
        "/Library/Fonts/Arial Unicode.ttf"
    };
    ui_font = GetFontDefault();
    owns_ui_font = 0;
    for (size_t i = 0; i < sizeof(candidates) / sizeof(candidates[0]); i++) {
        if (load_ui_font(candidates[i], 32)) break;
    }

    static const char *bold_candidates[] = {
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-M.ttf",
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
        "/home/linuxbrew/.linuxbrew/share/fonts/dejavu/DejaVuSans-Bold.ttf",
        "/opt/homebrew/share/fonts/dejavu/DejaVuSans-Bold.ttf"
    };
    ui_font_bold = ui_font;
    owns_ui_font_bold = 0;
    for (size_t i = 0;
         i < sizeof(bold_candidates) / sizeof(bold_candidates[0]); i++) {
        const char *path = bold_candidates[i];
        if (!FileExists(path)) continue;
        Font loaded = LoadFontEx(path, 32, NULL, 0);
        if (!IsFontValid(loaded) ||
            loaded.texture.id == GetFontDefault().texture.id) continue;
        SetTextureFilter(loaded.texture, TEXTURE_FILTER_BILINEAR);
        ui_font_bold = loaded;
        owns_ui_font_bold = 1;
        break;
    }
}

static Rectangle row(float height) {
    Rectangle r = {layout.x, layout.y, layout.width, height};
    layout.y += height + SM;
    return r;
}

static int hovered(Rectangle r) {
    return CheckCollisionPointRec(GetMousePosition(), r);
}

static void rounded(Rectangle r, float radius, Color fill, Color border) {
    float roundness = fminf(1.0f, radius * 2.0f / fminf(r.width, r.height));
    DrawRectangleRounded(r, roundness, 8, fill);
    DrawRectangleRoundedLinesEx(r, roundness, 8, 1.0f, border);
}

static void label_at_font(Font font, const char *text, Rectangle r,
                          int size, Color color) {
    if (!text) text = "";
    Vector2 measured = MeasureTextEx(font, text, (float)size, 0);
    Vector2 position = {r.x, r.y + (r.height - measured.y) * 0.5f};
    DrawTextEx(font, text, position, (float)size, 0, color);
}

static void label_at(const char *text, Rectangle r, int size, Color color) {
    label_at_font(ui_font, text, r, size, color);
}

int vidya_open(int width, int height, const char *title) {
    if (IsWindowReady()) return 0;
    SetConfigFlags(FLAG_WINDOW_RESIZABLE | FLAG_VSYNC_HINT | FLAG_MSAA_4X_HINT);
    InitWindow(width, height, title ? title : "Vidya");
    load_platform_font();
    SetExitKey(KEY_NULL);
    return IsWindowReady() ? 1 : 0;
}

void vidya_close(void) {
    if (owns_ui_font) {
        UnloadFont(ui_font);
        owns_ui_font = 0;
    }
    if (owns_ui_font_bold) {
        UnloadFont(ui_font_bold);
        owns_ui_font_bold = 0;
    }
    if (IsWindowReady()) CloseWindow();
}
int vidya_should_close(void) { return WindowShouldClose(); }
void vidya_set_target_fps(int fps) { SetTargetFPS(fps); }
void vidya_set_mode(int value) { mode = value == VIDYA_LIGHT ? VIDYA_LIGHT : VIDYA_DARK; }
int vidya_get_mode(void) { return mode; }
int vidya_load_font(const char *path, int atlas_size) {
    return load_ui_font(path, atlas_size);
}

void vidya_begin_frame(void) {
    BeginDrawing();
    ClearBackground(palette().window_bg);
    next_id = 0;
    card_depth = 0;
}

void vidya_end_frame(void) { EndDrawing(); }

void vidya_page_begin(float max_width) {
    float available = (float)GetScreenWidth() - PAGE * 2;
    layout.width = max_width > 0 ? fminf(max_width, available) : available;
    layout.x = ((float)GetScreenWidth() - layout.width) * 0.5f;
    layout.y = PAGE;
}

void vidya_page_end(void) {}

void vidya_card_begin(void) {
    if (card_depth >= 16) return;
    Palette p = palette();
    Color parent_fill = card_depth > 0 ? p.card_bg : p.window_bg;
    cards[card_depth++] = (Card){layout, layout.y, parent_fill};
    /* Paint ahead to the bottom of the view. card_end erases the unused tail
     * before outlining the final bounds. This gives immediate-mode callers an
     * opaque surface without requiring them to predict content height. */
    Rectangle surface = {
        layout.x, layout.y, layout.width, (float)GetScreenHeight() - layout.y
    };
    float roundness = fminf(1.0f, RADIUS_MD * 2.0f /
                                  fminf(surface.width, surface.height));
    DrawRectangleRounded(surface, roundness, 8, p.card_bg);
    layout.x += MD;
    layout.y += MD;
    layout.width -= MD * 2;
}

void vidya_card_end(void) {
    if (card_depth <= 0) return;
    Card card = cards[--card_depth];
    float bottom = layout.y - SM + MD;
    Rectangle bounds = {card.parent.x, card.top, card.parent.width, bottom - card.top};
    Rectangle tail = {
        card.parent.x, bottom, card.parent.width,
        fmaxf(0, (float)GetScreenHeight() - bottom)
    };
    DrawRectangleRec(tail, card.parent_fill);
    float roundness = fminf(1.0f, RADIUS_MD * 2.0f /
                                  fminf(bounds.width, bounds.height));
    DrawRectangleRoundedLinesEx(bounds, roundness, 8, 1.0f,
                                palette().border_soft);
    layout = card.parent;
    layout.y = bottom + SM;
}

void vidya_gap(float pixels) { layout.y += pixels; }

void vidya_separator(void) {
    Rectangle r = row(1);
    DrawRectangleRec(r, palette().border_soft);
}

static void text_role(Font font, const char *text, int size,
                      Color color, float height) {
    label_at_font(font, text, row(height), size, color);
}

void vidya_title(const char *text) {
    text_role(ui_font_bold, text, 20, palette().text, 26);
}
void vidya_title_2(const char *text) {
    text_role(ui_font_bold, text, 16, palette().text, 22);
}
void vidya_body(const char *text) {
    text_role(ui_font, text, 14, palette().text, 20);
}
void vidya_dim_label(const char *text) {
    text_role(ui_font, text, 12, palette().text_secondary, 18);
}

int vidya_button(const char *label, int kind) {
    Palette p = palette();
    Vector2 measured = MeasureTextEx(ui_font, label ? label : "", 14, 0);
    float width = fminf(layout.width, fmaxf(112, measured.x + MD * 2));
    Rectangle full_row = row(CONTROL_H);
    Rectangle r = {full_row.x, full_row.y, width, full_row.height};
    int over = hovered(r);
    int down = over && IsMouseButtonDown(MOUSE_BUTTON_LEFT);
    Color fill = down ? p.button_active : (over ? p.button_hover : p.button_bg);
    Color fg = p.button_fg;
    Color border = p.border_soft;
    if (kind == VIDYA_BUTTON_PRIMARY) {
        fill = down ? p.accent_active : (over ? p.accent_hover : p.accent);
        fg = p.accent_fg;
        border = fill;
    } else if (kind == VIDYA_BUTTON_DESTRUCTIVE) {
        fill = over ? ColorBrightness(p.destructive, 0.12f) : p.destructive;
        fg = WHITE;
        border = fill;
    }
    rounded(r, RADIUS_MD, fill, border);
    Rectangle tr = {r.x + (r.width - measured.x) / 2, r.y, measured.x, r.height};
    label_at(label, tr, 14, fg);
    return over && IsMouseButtonReleased(MOUSE_BUTTON_LEFT);
}

int vidya_checkbox(const char *label, int *checked) {
    if (!checked) return 0;
    Palette p = palette();
    Rectangle r = row(CONTROL_H);
    Rectangle box = {r.x, r.y + 7, 20, 20};
    int over = hovered(r);
    int changed = over && IsMouseButtonReleased(MOUSE_BUTTON_LEFT);
    if (changed) *checked = !*checked;
    Color fill = *checked ? (over ? p.accent_hover : p.accent)
                          : (over ? p.button_hover : p.button_bg);
    rounded(box, RADIUS_SM, fill, *checked ? fill : p.border_soft);
    if (*checked) {
        Vector2 points[3] = {
            {box.x + 4, box.y + 10}, {box.x + 8, box.y + 14}, {box.x + 16, box.y + 6}
        };
        DrawLineEx(points[0], points[1], 2.2f, p.accent_fg);
        DrawLineEx(points[1], points[2], 2.2f, p.accent_fg);
    }
    Rectangle tr = {r.x + 30, r.y, r.width - 30, r.height};
    label_at(label, tr, 14, p.text);
    return changed;
}

int vidya_checkbox_value(const char *label, int checked) {
    vidya_checkbox(label, &checked);
    return checked ? 1 : 0;
}

void vidya_status(const char *label, int live) {
    Palette p = palette();
    Rectangle r = row(20);
    Vector2 center = {r.x + 5, r.y + 10};
    Color color = live ? p.success : p.text_secondary;
    if (live) DrawCircleV(center, 4, color);
    else DrawCircleLinesV(center, 4, color);
    Rectangle tr = {r.x + 16, r.y, r.width - 16, r.height};
    label_at(label, tr, 14, p.text);
}

int vidya_text_field(char *text, size_t capacity, const char *placeholder) {
    if (!text || capacity == 0) return 0;
    Palette p = palette();
    Rectangle r = row(CONTROL_H);
    int id = ++next_id;
    if (hovered(r) && IsMouseButtonPressed(MOUSE_BUTTON_LEFT)) active_field = id;
    int active = active_field == id;
    int changed = 0;
    if (active) {
        int ch;
        size_t len = strlen(text);
        while ((ch = GetCharPressed()) > 0) {
            if (ch >= 32 && ch <= 0x7e && len + 1 < capacity) {
                text[len++] = (char)ch;
                text[len] = '\0';
                changed = 1;
            }
        }
        if (IsKeyPressed(KEY_BACKSPACE) && len > 0) {
            text[len - 1] = '\0';
            changed = 1;
        }
    }
    rounded(r, RADIUS_MD, p.view_bg, active ? p.accent : p.border_soft);
    const char *shown = text[0] ? text : (placeholder ? placeholder : "");
    Color color = text[0] ? p.text : p.text_disabled;
    Rectangle tr = {r.x + MD, r.y, r.width - MD * 2, r.height};
    label_at(shown, tr, 14, color);
    return changed;
}
