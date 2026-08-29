#include "vidya.h"

int main(void) {
    int enabled = 1;
    char name[128] = "";
    if (!vidya_open(720, 560, "Vidya · raylib")) return 1;
    vidya_set_target_fps(60);

    while (!vidya_should_close()) {
        vidya_begin_frame();
        vidya_page_begin(560);
        vidya_title("Vidya");
        vidya_dim_label("GNOME/HIG-inspired UI for raylib");
        vidya_gap(12);
        vidya_card_begin();
        vidya_title_2("Actions");
        vidya_body("Semantic controls retain Vidya's palette and spacing.");
        vidya_button("Primary action", VIDYA_BUTTON_PRIMARY);
        vidya_button("Default action", VIDYA_BUTTON_DEFAULT);
        vidya_button("Destructive action", VIDYA_BUTTON_DESTRUCTIVE);
        vidya_card_end();
        vidya_gap(12);
        vidya_card_begin();
        vidya_title_2("Forms");
        vidya_text_field(name, sizeof(name), "Your name");
        vidya_checkbox("Sync preferences", &enabled);
        vidya_status(enabled ? "Connected" : "Offline", enabled);
        vidya_card_end();
        vidya_page_end();
        vidya_end_frame();
    }
    vidya_close();
    return 0;
}
