//! Window, GL context, and the caller-driven frame loop.
//!
//! The C ABI is pull-style: the caller owns the loop and calls
//! [`App::begin_frame`] / [`App::end_frame`] around its own widget calls.
//! `eframe` inverts that — it owns the loop and calls the app — so this backend
//! drives `winit` with `pump_app_events` and paints through `egui_glow`,
//! keeping the ABI (and every consumer of it) unchanged.
//!
//! One thing does **not** get to move outside the event loop: the present.
//! Wayland gives a surface one buffer per frame callback, and winit tracks
//! those callbacks itself. Presenting from outside its `RedrawRequested`
//! dispatch desynchronizes that bookkeeping — the compositor stops calling
//! back, the next `swap_buffers` blocks inside EGL with the session waiting on
//! it, and the whole desktop stalls for seconds. So [`App::end_frame`] hands
//! the finished frame to [`Handler`] and pumps until it is painted from inside
//! the callback, which is also what paces the caller's loop.

use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::ViewportId;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::GetGlDisplay as _;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow as _};
use vidya_core::Theme;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::EventLoopExtPumpEvents as _;
use winit::raw_window_handle::HasWindowHandle as _;
use winit::window::{Window, WindowId};

use crate::ui::Stack;

/// How long `vidya_open` waits for the platform to hand us a window.
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a finished frame waits for a frame callback before being dropped.
/// A window nobody is compositing (minimized, on another workspace) simply
/// stops presenting; the caller's loop keeps running.
const PRESENT_TIMEOUT: Duration = Duration::from_millis(250);

/// Pacing floor, so a caller that never sets a target FPS still yields.
const DEFAULT_FRAME_BUDGET: Duration = Duration::from_micros(16_666);

/// Build the event loop, preferring X11 where both backends exist.
///
/// Native Wayland does not survive this ABI's shape: the caller owns the loop,
/// so winit is driven with `pump_app_events`, and a surface driven that way
/// stops receiving frame callbacks after its first commit — the window never
/// gets a second frame and the session stalls behind it. Under X11 (XWayland
/// included) presentation does not depend on those callbacks, and the same loop
/// runs at full frame rate. Falls back to the default backend when X11 is
/// unavailable, so a compositor without XWayland still gets a window.
#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
fn build_event_loop() -> Result<EventLoop<()>, String> {
    use winit::platform::x11::EventLoopBuilderExtX11 as _;

    if let Ok(el) = EventLoop::builder().with_x11().build() {
        return Ok(el);
    }
    eprintln!("vidya: X11 unavailable; falling back to Wayland (expect stalls)");
    EventLoop::builder()
        .build()
        .map_err(|e| format!("event loop: {e}"))
}

/// Android has no display connection to choose: the activity already owns one,
/// and winit reaches it through the handle the glue was started with. Without
/// that handle there is no event loop to build at all, which is why
/// `libvidya.so` is the NativeActivity's own library — see `android.rs`.
#[cfg(target_os = "android")]
fn build_event_loop() -> Result<EventLoop<()>, String> {
    use winit::platform::android::EventLoopBuilderExtAndroid as _;

    let app = crate::android::android_app()
        .ok_or_else(|| "no AndroidApp: vidya_open ran outside android_main".to_owned())?;
    EventLoop::builder()
        .with_android_app(app)
        .build()
        .map_err(|e| format!("event loop: {e}"))
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
fn build_event_loop() -> Result<EventLoop<()>, String> {
    EventLoop::builder()
        .build()
        .map_err(|e| format!("event loop: {e}"))
}

/// Window, GL surface, painter, and egui input translation.
///
/// Created on the first `resumed`, the only point where winit guarantees a
/// usable display connection on every platform.
struct Gl {
    window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    painter: egui_glow::Painter,
    winit_state: egui_winit::State,
}

impl Gl {
    fn create(
        el: &ActiveEventLoop,
        egui_ctx: &egui::Context,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let attrs = Window::default_attributes()
            .with_title(title)
            .with_inner_size(LogicalSize::new(width as f64, height as f64));

        let (window, config) = DisplayBuilder::new()
            .with_window_attributes(Some(attrs))
            .build(
                el,
                ConfigTemplateBuilder::new().with_alpha_size(0),
                // egui does its own anti-aliasing; this just avoids picking a
                // degenerate config.
                |configs| {
                    configs
                        .reduce(|best, c| {
                            if c.num_samples() > best.num_samples() {
                                c
                            } else {
                                best
                            }
                        })
                        .expect("no GL config")
                },
            )
            .map_err(|e| format!("GL display: {e}"))?;
        let window = window.ok_or_else(|| "no window was created".to_owned())?;

        let raw = window
            .window_handle()
            .map_err(|e| format!("window handle: {e}"))?
            .as_raw();
        let display = config.display();

        // Desktop GL first, GLES second — the order eframe uses.
        let context = unsafe {
            display
                .create_context(&config, &ContextAttributesBuilder::new().build(Some(raw)))
                .or_else(|_| {
                    display.create_context(
                        &config,
                        &ContextAttributesBuilder::new()
                            .with_context_api(ContextApi::Gles(None))
                            .build(Some(raw)),
                    )
                })
                .map_err(|e| format!("GL context: {e}"))?
        };

        let surface_attrs = window
            .build_surface_attributes(SurfaceAttributesBuilder::new())
            .map_err(|e| format!("surface attributes: {e}"))?;
        let surface = unsafe {
            display
                .create_window_surface(&config, &surface_attrs)
                .map_err(|e| format!("GL surface: {e}"))?
        };
        let context = context
            .make_current(&surface)
            .map_err(|e| format!("make current: {e}"))?;
        if let Err(e) = surface.set_swap_interval(
            &context,
            SwapInterval::Wait(std::num::NonZeroU32::new(1).expect("nonzero")),
        ) {
            eprintln!("vidya: vsync unavailable ({e})");
        }

        let glow_ctx = unsafe {
            glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s).cast())
        };
        let painter = egui_glow::Painter::new(Arc::new(glow_ctx), "", None, false)
            .map_err(|e| format!("painter: {e}"))?;

        let winit_state = egui_winit::State::new(
            egui_ctx.clone(),
            ViewportId::ROOT,
            &window,
            None,
            window.theme(),
            Some(painter.max_texture_side()),
        );

        Ok(Self {
            window,
            surface,
            context,
            painter,
            winit_state,
        })
    }
}

/// A tessellated frame waiting for the compositor to ask for it.
struct PaintJob {
    primitives: Vec<egui::ClippedPrimitive>,
    textures_delta: egui::TexturesDelta,
    pixels_per_point: f32,
    clear: [f32; 4],
}

/// winit event sink. Owns everything that survives between frames.
struct Handler {
    egui_ctx: egui::Context,
    theme: Theme,
    gl: Option<Gl>,
    title: String,
    width: u32,
    height: u32,
    should_close: bool,
    /// Set when window creation fails, so `vidya_open` can report it.
    error: Option<String>,
    /// Whether the compositor is ready for another buffer. Starts open: the
    /// initial configure entitles us to the first frame, and the frame callback
    /// that arms every later one is only requested *by* presenting — waiting
    /// for it before the first present deadlocks.
    may_present: bool,
    frames: u32,
    /// `VIDYA_CAPTURE=<path>`: dump one painted frame, then stop.
    capture: Option<String>,
}

impl Handler {
    /// Paint and present. Only ever called from inside `RedrawRequested`.
    fn present(&mut self, job: PaintJob) {
        let Some(gl) = self.gl.as_mut() else {
            return;
        };
        let size = gl.window.inner_size();
        let dims = [size.width.max(1), size.height.max(1)];

        gl.painter.clear(dims, job.clear);
        gl.painter.paint_and_update_textures(
            dims,
            job.pixels_per_point,
            &job.primitives,
            &job.textures_delta,
        );

        self.frames += 1;
        // Third frame: fonts and layout have settled by then.
        if self.frames == 3 {
            if let Some(path) = self.capture.take() {
                capture_frame(gl, dims, &path);
            }
        }

        // Order matters. `pre_present_notify` lets winit attach its frame
        // callback to the commit that `swap_buffers` is about to make, and the
        // redraw request that arms the *next* callback only counts once that
        // commit has happened.
        gl.window.pre_present_notify();
        if let Err(e) = gl.surface.swap_buffers(&gl.context) {
            eprintln!("vidya: swap_buffers failed: {e}");
        }
        gl.window.request_redraw();
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gl.is_some() {
            return;
        }
        match Gl::create(el, &self.egui_ctx, &self.title, self.width, self.height) {
            Ok(gl) => {
                vidya_core::apply(&self.egui_ctx, &self.theme);
                self.gl = Some(gl);
            }
            Err(e) => {
                self.error = Some(e);
                self.should_close = true;
            }
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(gl) = self.gl.as_mut() {
            // egui sees every event, including the ones handled below.
            let _ = gl.winit_state.on_window_event(&gl.window, &event);

            if let WindowEvent::Resized(size) = event {
                if size.width > 0 && size.height > 0 {
                    gl.window.resize_surface(&gl.surface, &gl.context);
                }
            }
        }

        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => self.should_close = true,
            WindowEvent::RedrawRequested => self.may_present = true,
            _ => {}
        }
    }
}

/// One UI context per process, matching the ABI's window model.
pub struct App {
    event_loop: EventLoop<()>,
    handler: Handler,
    /// Live only between `begin_frame` and `end_frame`.
    pub stack: Stack,
    frame_budget: Duration,
    frame_started: Instant,
    /// Frames skipped because nothing was compositing the window.
    dropped: u32,
    font_generation: u32,
}

impl App {
    pub fn open(width: i32, height: i32, title: &str) -> Result<Self, String> {
        let event_loop = build_event_loop()?;

        let mut app = Self {
            event_loop,
            handler: Handler {
                egui_ctx: egui::Context::default(),
                theme: Theme::dark(),
                gl: None,
                title: title.to_owned(),
                width: width.max(1) as u32,
                height: height.max(1) as u32,
                should_close: false,
                error: None,
                may_present: true,
                frames: 0,
                capture: std::env::var("VIDYA_CAPTURE").ok(),
            },
            stack: Stack::default(),
            frame_budget: DEFAULT_FRAME_BUDGET,
            frame_started: Instant::now(),
            dropped: 0,
            font_generation: 0,
        };

        // Pump until the platform resumes us and the window exists.
        let deadline = Instant::now() + OPEN_TIMEOUT;
        while app.handler.gl.is_none() && app.handler.error.is_none() {
            if Instant::now() > deadline {
                return Err("timed out waiting for a window".to_owned());
            }
            app.pump(Duration::from_millis(10));
        }
        match app.handler.error.take() {
            Some(e) => Err(e),
            None => Ok(app),
        }
    }

    fn pump(&mut self, timeout: Duration) {
        self.event_loop
            .pump_app_events(Some(timeout), &mut self.handler);
    }

    pub fn should_close(&mut self) -> bool {
        self.pump(Duration::ZERO);
        self.handler.should_close
    }

    pub fn theme(&self) -> &Theme {
        &self.handler.theme
    }

    /// The innermost open node plus the live theme.
    ///
    /// Returned together because widget calls need both, and they live in
    /// different fields — splitting the borrow here keeps the call sites plain.
    pub fn ui(&mut self) -> Option<(&mut egui::Ui, &Theme)> {
        let theme = &self.handler.theme;
        self.stack.top().map(|ui| (ui, theme))
    }

    pub fn set_mode(&mut self, mode: vidya_core::Mode) {
        self.handler.theme = match mode {
            vidya_core::Mode::Dark => Theme::dark(),
            vidya_core::Mode::Light => Theme::light(),
        };
        vidya_core::apply(&self.handler.egui_ctx, &self.handler.theme);
    }

    pub fn set_target_fps(&mut self, fps: i32) {
        self.frame_budget = match fps {
            f if f > 0 => Duration::from_secs_f64(1.0 / f as f64),
            _ => DEFAULT_FRAME_BUDGET,
        };
    }

    /// Install a UI font as the highest-priority proportional family.
    ///
    /// The symbol fallback installed by [`vidya_core::apply`] stays in place,
    /// so punctuation and block art keep rendering.
    pub fn load_font(&mut self, path: &str) -> bool {
        let Ok(bytes) = std::fs::read(path) else {
            return false;
        };
        self.font_generation += 1;
        // Unique name per load: egui skips a name it already has.
        let name = format!("vidya-ui-{}", self.font_generation);
        self.handler
            .egui_ctx
            .add_font(egui::epaint::text::FontInsert::new(
                &name,
                egui::FontData::from_owned(bytes),
                vec![egui::epaint::text::InsertFontFamily {
                    family: egui::FontFamily::Proportional,
                    priority: egui::epaint::text::FontPriority::Highest,
                }],
            ));
        true
    }

    /// Drain pending input and open an egui pass with a root [`egui::Ui`].
    pub fn begin_frame(&mut self) {
        self.pump(Duration::ZERO);
        self.frame_started = Instant::now();

        if self.handler.gl.is_none() || self.stack.is_active() {
            // No window, or the caller skipped `vidya_end_frame`.
            return;
        }
        let Some(gl) = self.handler.gl.as_mut() else {
            return;
        };

        let input = gl.winit_state.take_egui_input(&gl.window);
        let ctx = &self.handler.egui_ctx;
        ctx.begin_pass(input);
        self.stack.push_root(ctx);
    }

    /// Close the pass, then hand the frame to the event loop to present.
    pub fn end_frame(&mut self) {
        if !self.stack.is_active() {
            return;
        }
        // Close anything the caller left open (a missing `vidya_card_end`).
        self.stack.unwind();
        if self.handler.gl.is_none() {
            return;
        }

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = self.handler.egui_ctx.end_pass();
        let primitives = self.handler.egui_ctx.tessellate(shapes, pixels_per_point);
        // Gamma, not linear: `Painter::clear` hands these straight to
        // `glClearColor` against an sRGB framebuffer, so a `Rgba::from`
        // conversion here would be applied twice and the window would clear to
        // near-black instead of the palette's charcoal.
        let clear = self
            .handler
            .theme
            .palette
            .window_bg
            .to_normalized_gamma_f32();

        if let Some(gl) = self.handler.gl.as_mut() {
            gl.winit_state
                .handle_platform_output(&gl.window, platform_output);
        }

        // Wait for the compositor to want a frame, dispatching events while we
        // wait — never inside `swap_buffers`, which would park the thread that
        // owes the compositor its replies and stall the session. This wait is
        // also what paces the caller's loop to the display.
        let deadline = Instant::now() + PRESENT_TIMEOUT;
        while !self.handler.may_present && Instant::now() < deadline {
            self.pump(Duration::from_millis(2));
        }

        if self.handler.may_present {
            self.handler.may_present = false;
            self.handler.present(PaintJob {
                primitives,
                textures_delta,
                pixels_per_point,
                clear,
            });
        } else {
            // Nobody is compositing this window (minimized, another workspace).
            // Drop the frame rather than force a present that would block.
            self.dropped += 1;
            if self.dropped == 1 {
                eprintln!("vidya: window is not being composited; dropping frames");
            }
        }

        // Floor for platforms that do not throttle presents at all.
        if let Some(left) = self.frame_budget.checked_sub(self.frame_started.elapsed()) {
            std::thread::sleep(left);
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.stack.unwind();
        if let Some(gl) = self.handler.gl.as_mut() {
            gl.painter.destroy();
        }
    }
}

/// Write the painted framebuffer to a binary PPM.
///
/// A rendering backend is otherwise unverifiable where the compositor refuses
/// screenshots, and in CI where there is nobody to look. Off unless
/// `VIDYA_CAPTURE` names a path.
fn capture_frame(gl: &Gl, dims: [u32; 2], path: &str) {
    use glow::HasContext as _;
    use std::io::Write as _;

    let [w, h] = dims;
    let mut rgba = vec![0u8; (w as usize) * (h as usize) * 4];
    unsafe {
        // Drain the pipeline: the pixels need not exist yet.
        gl.painter.gl().finish();
        gl.painter.gl().read_pixels(
            0,
            0,
            w as i32,
            h as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut rgba)),
        );
    }

    let mut out = Vec::with_capacity((w as usize) * (h as usize) * 3 + 32);
    out.extend_from_slice(format!("P6\n{w} {h}\n255\n").as_bytes());
    // GL origin is bottom-left; PPM is top-down.
    for row in (0..h as usize).rev() {
        let start = row * w as usize * 4;
        for px in rgba[start..start + w as usize * 4].chunks_exact(4) {
            out.extend_from_slice(&px[..3]);
        }
    }

    match std::fs::File::create(path).and_then(|mut f| f.write_all(&out)) {
        Ok(()) => eprintln!("vidya: wrote {path}"),
        Err(e) => eprintln!("vidya: capture failed: {e}"),
    }
}
