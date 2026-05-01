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
        event::WindowEvent,
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        window::{Window, WindowId},
    },
};

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub title: &'static str,
    pub size: PhysicalSize<u32>,
}

type Animation = Box<dyn FnMut(&mut Scene, f32)>;

pub struct App {
    options: Options,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    scene: Option<Scene>,
    scene_render_cache: SceneRenderCache,
    animation: Option<Animation>,
    frame_clock: FrameClock,
    needs_redraw: bool,
}

impl App {
    pub fn new(options: Options) -> Self {
        Self {
            options,
            window: None,
            renderer: None,
            scene: None,
            scene_render_cache: SceneRenderCache::new(),
            animation: None,
            frame_clock: FrameClock::default(),
            needs_redraw: false,
        }
    }

    pub fn render(&mut self, scene: Scene) {
        self.scene_render_cache = SceneRenderCache::new();
        self.scene = Some(scene);
        self.needs_redraw = true;
    }

    pub fn animate(&mut self, animation: impl FnMut(&mut Scene, f32) + 'static) -> &mut Self {
        self.animation = Some(Box::new(animation));
        self.needs_redraw = true;
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
            .field("frame_clock", &self.frame_clock)
            .field("needs_redraw", &self.needs_redraw)
            .finish()
    }
}

#[derive(Debug, Default)]
struct FrameClock {
    last_frame_time: Option<Instant>,
    fps_frame_count: u32,
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

    fn log_presented_frame(&mut self, now: Instant) {
        self.fps_frame_count += 1;
        let last_log_time = self.fps_last_log_time.get_or_insert(now);
        let elapsed = now.duration_since(*last_log_time);
        if elapsed >= Duration::from_secs(1) {
            log::debug!(
                "fps: {:.1}",
                self.fps_frame_count as f32 / elapsed.as_secs_f32()
            );
            self.fps_frame_count = 0;
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
                renderer.resize(size);
                self.needs_redraw = true;
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
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
        if let Some(animation) = &mut self.animation {
            animation(scene, self.frame_clock.frame_delta(now));
        }

        let scene_render_data = prepare_scene(&mut self.scene_render_cache, scene);
        let render_status = renderer.render(&scene_render_data);
        self.needs_redraw = matches!(render_status, RenderStatus::NeedsRedraw);
        if matches!(render_status, RenderStatus::Presented) {
            self.frame_clock.log_presented_frame(now);
        }
    }
}
