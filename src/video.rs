//! Inline video preview / muted H.264-in-MP4 player for egui.
//!
//! Host apps fetch bytes (SSRF-safe), then mount [`video_player`] with a
//! [`VideoPlayerState`]. Decode is behind the `video` cargo feature so default
//! `vidya` stays light (theme-only).

use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

use egui::{
    Align2, Color32, ColorImage, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, TextureHandle,
    TextureOptions, Ui, Vec2,
};

use crate::Theme;

#[cfg(feature = "video")]
mod avcc;
#[cfg(feature = "video")]
mod decode;

/// Options for [`video_player`].
#[derive(Debug, Clone)]
pub struct VideoPlayerOpts {
    /// Max width of the player surface.
    pub max_width: f32,
    /// Max height of the player surface.
    pub max_height: f32,
    /// Footer label (filename / host).
    pub title: Option<String>,
    /// When set, a failed / unsupported decode falls back to opening this URL.
    pub open_url_on_unsupported: Option<String>,
}

impl Default for VideoPlayerOpts {
    fn default() -> Self {
        Self {
            max_width: 320.0,
            max_height: 288.0,
            title: None,
            open_url_on_unsupported: None,
        }
    }
}

/// Outcome of interacting with the player this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoPlayerAction {
    #[default]
    None,
    /// User asked to open the media externally (unsupported codec / explicit).
    OpenExternally,
}

/// Per-embed playback state. Store one of these per video URL (or message id).
#[derive(Default)]
pub struct VideoPlayerState {
    content_id: u64,
    #[cfg(feature = "video")]
    session: Option<decode::DecodeSession>,
    texture: Option<TextureHandle>,
    tex_size: (u32, u32),
    playing: bool,
    play_started: Option<Instant>,
    /// Elapsed media time when paused.
    paused_at: f64,
    error: Option<String>,
    unsupported: bool,
    loaded: bool,
}

impl std::fmt::Debug for VideoPlayerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoPlayerState")
            .field("content_id", &self.content_id)
            .field("playing", &self.playing)
            .field("error", &self.error)
            .field("unsupported", &self.unsupported)
            .field("loaded", &self.loaded)
            .finish()
    }
}

impl VideoPlayerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn unsupported(&self) -> bool {
        self.unsupported
    }

    /// Load (or reload) MP4 bytes. Idempotent when `id` matches the current content.
    pub fn load_bytes(&mut self, ctx: &egui::Context, id: impl Hash, bytes: Arc<[u8]>) {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        // Also mix length so same path with new bytes reloads.
        bytes.len().hash(&mut hasher);
        let content_id = hasher.finish();
        if self.loaded && self.content_id == content_id {
            return;
        }
        self.reset();
        self.content_id = content_id;

        #[cfg(feature = "video")]
        {
            match decode::DecodeSession::open(bytes) {
                Ok(mut session) => {
                    if let Some((w, h, rgba)) = session.frame.take() {
                        self.upload_frame(ctx, w, h, &rgba);
                        session.frame = Some((w, h, rgba));
                    }
                    self.session = Some(session);
                    self.loaded = true;
                }
                Err(e) => {
                    self.unsupported = true;
                    self.error = Some(e);
                    self.loaded = true;
                }
            }
        }

        #[cfg(not(feature = "video"))]
        {
            let _ = (ctx, bytes);
            self.unsupported = true;
            self.error = Some("Video decode feature not enabled".into());
            self.loaded = true;
        }
    }

    pub fn clear(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        #[cfg(feature = "video")]
        {
            self.session = None;
        }
        self.texture = None;
        self.tex_size = (0, 0);
        self.playing = false;
        self.play_started = None;
        self.paused_at = 0.0;
        self.error = None;
        self.unsupported = false;
        self.loaded = false;
        self.content_id = 0;
    }

    fn upload_frame(&mut self, ctx: &egui::Context, w: u32, h: u32, rgba: &[u8]) {
        let color = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba);
        match &mut self.texture {
            Some(tex) if self.tex_size == (w, h) => {
                tex.set(color, TextureOptions::LINEAR);
            }
            _ => {
                let name = format!("vidya-video-{}", self.content_id);
                self.texture = Some(ctx.load_texture(name, color, TextureOptions::LINEAR));
                self.tex_size = (w, h);
            }
        }
    }

    fn media_time(&self) -> f64 {
        if self.playing {
            let started = self.play_started.unwrap_or_else(Instant::now);
            self.paused_at + started.elapsed().as_secs_f64()
        } else {
            self.paused_at
        }
    }

    fn toggle_play(&mut self, ctx: &egui::Context) {
        if self.unsupported {
            return;
        }
        if self.playing {
            self.paused_at = self.media_time();
            self.playing = false;
            self.play_started = None;
        } else {
            #[cfg(feature = "video")]
            {
                if let Some(session) = self.session.as_ref() {
                    if self.paused_at >= session.duration.as_secs_f64() {
                        self.paused_at = 0.0;
                    }
                }
            }
            self.playing = true;
            self.play_started = Some(Instant::now());
            ctx.request_repaint();
        }
    }

    #[cfg(feature = "video")]
    fn tick_decode(&mut self, ctx: &egui::Context) {
        if !self.playing {
            return;
        }
        let t = self.paused_at
            + self
                .play_started
                .map(|s| s.elapsed().as_secs_f64())
                .unwrap_or(0.0);

        let Some(session) = self.session.as_mut() else {
            return;
        };
        if let Err(e) = session.seek_playhead(t) {
            self.error = Some(e);
            self.playing = false;
            return;
        }
        let frame = session.frame.clone();
        let ended = session.ended(t);
        let duration = session.duration.as_secs_f64();

        if let Some((w, h, rgba)) = frame {
            self.upload_frame(ctx, w, h, &rgba);
        }
        if ended {
            self.playing = false;
            self.play_started = None;
            self.paused_at = duration;
        } else {
            ctx.request_repaint();
        }
    }
}

/// Draw a 16:9 (or source-aspect) video surface with play/pause.
///
/// Call [`VideoPlayerState::load_bytes`] first when media bytes are ready.
pub fn video_player(
    ui: &mut Ui,
    theme: &Theme,
    state: &mut VideoPlayerState,
    opts: &VideoPlayerOpts,
) -> (egui::Response, VideoPlayerAction) {
    let p = &theme.palette;
    let sp = &theme.spacing;

    #[cfg(feature = "video")]
    state.tick_decode(ui.ctx());

    let max_w = ui.available_width().min(opts.max_width).max(120.0);
    let (src_w, src_h) = if state.tex_size.0 > 0 && state.tex_size.1 > 0 {
        (state.tex_size.0 as f32, state.tex_size.1 as f32)
    } else {
        (16.0, 9.0)
    };
    let height = (max_w * src_h / src_w)
        .min(opts.max_height)
        .max(72.0);
    let size = Vec2::new(max_w, height);

    let mut action = VideoPlayerAction::None;
    let mut clicked = false;

    let frame_resp = egui::Frame::new()
        .fill(Color32::from_rgb(12, 12, 14))
        .stroke(Stroke::new(1.0_f32, p.border_soft))
        .corner_radius(sp.radius_sm)
        .show(ui, |ui| {
            let (rect, sense_resp) = ui.allocate_exact_size(size, Sense::click());
            clicked = sense_resp.clicked();

            if let Some(tex) = state.texture.as_ref() {
                egui::Image::new((tex.id(), size)).paint_at(ui, rect);
            } else {
                ui.painter()
                    .rect_filled(rect, sp.radius_sm, Color32::from_rgb(18, 18, 22));
            }

            // Dim + play/pause affordance when not playing (or never started).
            if !state.playing {
                ui.painter().rect_filled(
                    rect,
                    sp.radius_sm,
                    Color32::from_rgba_unmultiplied(0, 0, 0, 48),
                );
                let play_r = (height * 0.18).clamp(16.0, 28.0);
                let center = rect.center();
                ui.painter()
                    .circle_filled(center, play_r, p.accent.gamma_multiply(0.92));
                let tri_w = play_r * 0.7;
                let tri_h = play_r * 0.85;
                let tip = Pos2::new(center.x + tri_w * 0.55, center.y);
                let top = Pos2::new(center.x - tri_w * 0.45, center.y - tri_h * 0.5);
                let bot = Pos2::new(center.x - tri_w * 0.45, center.y + tri_h * 0.5);
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![tip, bot, top],
                    Color32::from_rgb(255, 255, 255),
                    Stroke::NONE,
                ));
            }

            let footer = if let Some(err) = state.error.as_deref() {
                Some(err.to_string())
            } else if state.playing {
                Some("Playing…".into())
            } else {
                opts.title.clone()
            };
            if let Some(name) = footer {
                let foot_h = (theme.type_scale.caption + 10.0).min(height * 0.28);
                let foot = Rect::from_min_max(
                    Pos2::new(rect.left(), rect.bottom() - foot_h),
                    rect.right_bottom(),
                );
                let r = sp.radius_sm as u8;
                ui.painter().rect_filled(
                    foot,
                    egui::CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: r,
                        se: r,
                    },
                    Color32::from_rgba_unmultiplied(0, 0, 0, 160),
                );
                ui.painter().text(
                    Pos2::new(foot.left() + 8.0, foot.center().y),
                    Align2::LEFT_CENTER,
                    name,
                    FontId::proportional(theme.type_scale.caption),
                    Color32::from_rgb(230, 230, 235),
                );
            }

            let tip = if state.unsupported {
                state
                    .error
                    .as_deref()
                    .unwrap_or("Video not supported — open externally")
            } else if state.playing {
                "Pause"
            } else {
                "Play"
            };
            sense_resp.on_hover_text(tip).on_hover_cursor(CursorIcon::PointingHand);
        });

    if clicked {
        if state.unsupported {
            if opts.open_url_on_unsupported.is_some() {
                action = VideoPlayerAction::OpenExternally;
            }
        } else {
            state.toggle_play(ui.ctx());
        }
    }

    (frame_resp.response, action)
}
