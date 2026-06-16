use {
    super::renderer::{RenderStatus, Renderer},
    crate::{
        render::{SceneRenderCache, prepare_scene},
        scene::Scene,
    },
    anyhow::{Context, Result},
    std::{
        fmt,
        sync::Arc,
        time::{Duration, Instant},
    },
    winit::{
        application::ApplicationHandler,
        dpi::PhysicalSize,
        event::{Modifiers, MouseScrollDelta, WindowEvent},
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        window::{Window, WindowId},
    },
};

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub title: &'static str,
    pub size: PhysicalSize<u32>,
    pub use_sdf_text: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum InputEvent {
    MouseWheel { delta: [f32; 2], command: bool },
}

type Animation = Box<dyn FnMut(&mut Scene, f32)>;
type InputHandler = Box<dyn FnMut(&mut Scene, InputEvent)>;

pub struct App {
    options: Options,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    scene: Option<Scene>,
    scene_render_cache: SceneRenderCache,
    animation: Option<Animation>,
    input_handler: Option<InputHandler>,
    frame_clock: FrameClock,
    needs_redraw: bool,
    next_redraw_time: Option<Instant>,
    window_occluded: bool,
    modifiers: Modifiers,
}

impl App {
    pub fn new(options: Options) -> Self {
        Self {
            options,
            window: None,
            renderer: None,
            scene: None,
            scene_render_cache: SceneRenderCache::new(options.use_sdf_text),
            animation: None,
            input_handler: None,
            frame_clock: FrameClock::default(),
            needs_redraw: false,
            next_redraw_time: None,
            window_occluded: false,
            modifiers: Modifiers::default(),
        }
    }

    pub fn render(&mut self, scene: Scene) {
        self.scene_render_cache = SceneRenderCache::new(self.options.use_sdf_text);
        self.scene = Some(scene);
        self.needs_redraw = true;
    }

    pub fn animate(&mut self, animation: impl FnMut(&mut Scene, f32) + 'static) -> &mut Self {
        self.animation = Some(Box::new(animation));
        self.needs_redraw = true;
        self
    }

    pub fn input(
        &mut self,
        input_handler: impl FnMut(&mut Scene, InputEvent) + 'static,
    ) -> &mut Self {
        self.input_handler = Some(Box::new(input_handler));
        self
    }

    pub fn run(mut self) -> Result<()> {
        let event_loop = EventLoop::new().context("failed to create event loop")?;
        event_loop.run_app(&mut self).context("event loop failed")
    }
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("options", &self.options)
            .field("window", &self.window)
            .field("renderer", &self.renderer)
            .field("scene", &self.scene)
            .field("scene_render_cache", &self.scene_render_cache)
            .field("animation", &self.animation.is_some())
            .field("input_handler", &self.input_handler.is_some())
            .field("frame_clock", &self.frame_clock)
            .field("needs_redraw", &self.needs_redraw)
            .field("next_redraw_time", &self.next_redraw_time)
            .field("window_occluded", &self.window_occluded)
            .field("modifiers", &self.modifiers)
            .finish()
    }
}

#[derive(Debug, Default)]
struct FrameClock {
    last_frame_time: Option<Instant>,
    frame_time_total: Duration,
    prepare_time_total: Duration,
    shaping_time_total: Duration,
    render_time_total: Duration,
    fps_frame_count: u32,
    needs_redraw_count: u32,
    waiting_count: u32,
    fps_last_log_time: Option<Instant>,
}

impl FrameClock {
    fn frame_delta(&mut self, now: Instant) -> f32 {
        let dt = self
            .last_frame_time
            .map_or(0.0, |last_frame_time| {
                now.duration_since(last_frame_time).as_secs_f32()
            })
            .min(0.1);
        self.last_frame_time = Some(now);
        dt
    }

    fn reset(&mut self) {
        self.last_frame_time = None;
    }

    fn log_frame(
        &mut self,
        status: RenderStatus,
        frame_time: Duration,
        prepare_time: Duration,
        shaping_time: Duration,
        render_time: Duration,
        now: Instant,
    ) {
        match status {
            RenderStatus::Presented => self.fps_frame_count += 1,
            RenderStatus::NeedsRedraw => self.needs_redraw_count += 1,
            RenderStatus::Waiting => self.waiting_count += 1,
        }
        self.frame_time_total += frame_time;
        self.prepare_time_total += prepare_time;
        self.shaping_time_total += shaping_time;
        self.render_time_total += render_time;
        let last_log_time = self.fps_last_log_time.get_or_insert(now);
        let elapsed = now.duration_since(*last_log_time);
        if elapsed >= Duration::from_secs(1) {
            log::debug!(
                concat!(
                    "fps: {:.1}, frame: {:.2}ms, prepare: {:.2}ms, shape: {:.2}ms, ",
                    "render+present: {:.2}ms, ",
                    "status: presented={}, needs_redraw={}, waiting={}"
                ),
                self.fps_frame_count as f32 / elapsed.as_secs_f32(),
                self.frame_time_total.as_secs_f32() * 1000.0
                    / (self.fps_frame_count + self.needs_redraw_count + self.waiting_count) as f32,
                self.prepare_time_total.as_secs_f32() * 1000.0
                    / (self.fps_frame_count + self.needs_redraw_count + self.waiting_count) as f32,
                self.shaping_time_total.as_secs_f32() * 1000.0
                    / (self.fps_frame_count + self.needs_redraw_count + self.waiting_count) as f32,
                self.render_time_total.as_secs_f32() * 1000.0
                    / (self.fps_frame_count + self.needs_redraw_count + self.waiting_count) as f32,
                self.fps_frame_count,
                self.needs_redraw_count,
                self.waiting_count,
            );
            self.frame_time_total = Duration::ZERO;
            self.prepare_time_total = Duration::ZERO;
            self.shaping_time_total = Duration::ZERO;
            self.render_time_total = Duration::ZERO;
            self.fps_frame_count = 0;
            self.needs_redraw_count = 0;
            self.waiting_count = 0;
            self.fps_last_log_time = Some(now);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.options.title)
            .with_inner_size(self.options.size);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                log::error!("failed to create window: {err}");
                event_loop.exit();
                return;
            }
        };
        log_window_state(&window, "created");

        let mut renderer = match pollster::block_on(Renderer::new(window.clone())) {
            Ok(renderer) => renderer,
            Err(err) => {
                log::error!("{err:#}");
                event_loop.exit();
                return;
            }
        };
        renderer.init();
        if self.needs_redraw {
            window.request_redraw();
        }
        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = &self.window else {
            return;
        };
        if id != window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                let Some(renderer) = &mut self.renderer else {
                    return;
                };
                log_window_state(window, "resized");
                renderer.resize(size);
                self.needs_redraw = true;
                self.next_redraw_time = None;
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                log::debug!("window scale factor changed: {scale_factor:.2}");
                log_window_state(window, "scale factor changed");
                self.next_redraw_time = None;
            }
            WindowEvent::Occluded(occluded) => {
                log::debug!("window occluded: {occluded}");
                self.window_occluded = occluded;
                self.next_redraw_time = None;
                if !occluded {
                    self.needs_redraw = true;
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers,
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => [x, y],
                    MouseScrollDelta::PixelDelta(position) => {
                        [position.x as f32 / 120.0, position.y as f32 / 120.0]
                    }
                };
                let (Some(scene), Some(input_handler)) = (&mut self.scene, &mut self.input_handler)
                else {
                    return;
                };
                input_handler(
                    scene,
                    InputEvent::MouseWheel {
                        delta,
                        command: self.modifiers.state().super_key(),
                    },
                );
                self.needs_redraw = true;
                self.next_redraw_time = None;
                window.request_redraw();
            }
            WindowEvent::RedrawRequested if !self.window_occluded => self.redraw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_occluded {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }

        if let Some(next_redraw_time) = self.next_redraw_time {
            if Instant::now() < next_redraw_time {
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_redraw_time));
                return;
            }
            self.next_redraw_time = None;
        }

        event_loop.set_control_flow(ControlFlow::Wait);
        if (self.needs_redraw || (self.animation.is_some() && self.scene.is_some()))
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
}

impl App {
    fn redraw(&mut self) {
        let (Some(renderer), Some(scene)) = (&mut self.renderer, &mut self.scene) else {
            self.needs_redraw = false;
            self.frame_clock.reset();
            return;
        };

        let now = Instant::now();
        let frame_start = now;
        if let Some(animation) = &mut self.animation {
            animation(scene, self.frame_clock.frame_delta(now));
        }

        let prepare_start = Instant::now();
        let scene_render_data = prepare_scene(&mut self.scene_render_cache, scene);
        let prepare_time = prepare_start.elapsed();
        let render_start = Instant::now();
        let render_status = renderer.render(&scene_render_data);
        let render_time = render_start.elapsed();
        self.needs_redraw = matches!(render_status, RenderStatus::NeedsRedraw);
        self.next_redraw_time = match render_status {
            RenderStatus::Presented | RenderStatus::NeedsRedraw => None,
            RenderStatus::Waiting => Some(now + REDRAW_BACKOFF),
        };
        self.frame_clock.log_frame(
            render_status,
            frame_start.elapsed(),
            prepare_time,
            scene_render_data.shaping_time,
            render_time,
            now,
        );
    }
}

const REDRAW_BACKOFF: Duration = Duration::from_millis(100);

fn log_window_state(window: &Window, label: &str) {
    let physical_size = window.inner_size();
    let scale_factor = window.scale_factor();
    let logical_size = physical_size.to_logical::<f64>(scale_factor);
    if let Some(monitor) = window.current_monitor() {
        let monitor_size = monitor.size();
        let refresh_rate = monitor
            .refresh_rate_millihertz()
            .map_or("unknown".to_string(), |millihertz| {
                format!("{:.1}Hz", millihertz as f32 / 1000.0)
            });
        log::debug!(
            concat!(
                "window {}: physical={}x{}, logical={:.1}x{:.1}, scale={:.2}, ",
                "monitor={:?}, monitor_size={}x{}, monitor_scale={:.2}, monitor_refresh={}"
            ),
            label,
            physical_size.width,
            physical_size.height,
            logical_size.width,
            logical_size.height,
            scale_factor,
            monitor.name(),
            monitor_size.width,
            monitor_size.height,
            monitor.scale_factor(),
            refresh_rate,
        );
        return;
    }

    log::debug!(
        "window {label}: physical={}x{}, logical={:.1}x{:.1}, scale={:.2}, monitor=unknown",
        physical_size.width,
        physical_size.height,
        logical_size.width,
        logical_size.height,
        scale_factor,
    );
}
