use {
    crate::{
        font::FontSys,
        render::{SceneRenderData, Vertex},
        scene::Camera,
    },
    anyhow::{Context, Result},
    wgpu::util::DeviceExt,
    winit::{dpi::PhysicalSize, window::Window},
};

#[derive(Debug)]
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    resources: Option<RenderResources>,
}

#[derive(Debug)]
struct RenderResources {
    render_pipeline: wgpu::RenderPipeline,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    atlas_bind_group: wgpu::BindGroup,
    atlas_texture: wgpu::Texture,
    atlas_version: u64,
    surface_bind_group_layout: wgpu::BindGroupLayout,
    surfaces: Vec<SurfaceResources>,
}

#[derive(Debug)]
struct SurfaceResources {
    surface_bind_group: wgpu::BindGroup,
    surface_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    content_version: u64,
    vertex_capacity: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum RenderStatus {
    Presented,
    NeedsRedraw,
    Waiting,
}

impl Renderer {
    pub async fn new(window: std::sync::Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window)
            .context("failed to create wgpu surface")?;

        let adapter_request = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        });
        let adapter = adapter_request
            .await
            .context("failed to find a compatible GPU adapter")?;

        let device_request = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        });
        let (device, queue) = device_request
            .await
            .context("failed to create wgpu device")?;

        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by the selected adapter")?;
        let capabilities = surface.get_capabilities(&adapter);
        config.format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .context("surface does not support an sRGB format")?;
        log::debug!(
            "surface capabilities: formats={:?}, present_modes={:?}, alpha_modes={:?}, usages={:?}",
            capabilities.formats,
            capabilities.present_modes,
            capabilities.alpha_modes,
            capabilities.usages,
        );
        log_surface_config("created", &config);
        surface.configure(&device, &config);

        Ok(Self {
            surface,
            config,
            device,
            queue,
            resources: None,
        })
    }

    pub(crate) fn init(&mut self) {
        let device = &self.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../render/scene.wgsl").into()),
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let atlas_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("atlas bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let surface_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("surface bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[
                Some(&camera_bind_group_layout),
                Some(&atlas_bind_group_layout),
                Some(&surface_bind_group_layout),
            ],
            immediate_size: 0,
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Unorm8x4,
                        2 => Float32x2,
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera"),
            contents: bytemuck::cast_slice(&Camera::default().matrix(1.0)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph atlas"),
            size: wgpu::Extent3d {
                width: FontSys::ATLAS_SIZE as u32,
                height: FontSys::ATLAS_SIZE as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph atlas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas bind group"),
            layout: &atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        self.resources = Some(RenderResources {
            render_pipeline,
            camera_bind_group,
            camera_buffer,
            atlas_bind_group,
            atlas_texture,
            atlas_version: 0,
            surface_bind_group_layout,
            surfaces: Vec::new(),
        });
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        log_surface_config("resized", &self.config);
        self.surface.configure(&self.device, &self.config);
    }

    pub(crate) fn render(&mut self, scene: &SceneRenderData<'_>) -> RenderStatus {
        if self.config.width == 0 || self.config.height == 0 {
            return RenderStatus::Waiting;
        }

        // seems that get_current_texture waits for vsync, it takes many ms
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                log::debug!("surface texture unavailable: outdated/lost, reconfiguring");
                self.surface.configure(&self.device, &self.config);
                return RenderStatus::NeedsRedraw;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                log::debug!("surface texture unavailable: timeout/occluded");
                return RenderStatus::Waiting;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                log::debug!("surface texture unavailable: validation");
                return RenderStatus::Waiting;
            }
        };

        if let Some(resources) = &mut self.resources {
            let aspect = self.config.width as f32 / self.config.height as f32;
            self.queue.write_buffer(
                &resources.camera_buffer,
                0,
                bytemuck::cast_slice(&scene.camera.matrix(aspect)),
            );
            let atlas_version = scene.font_sys.atlas_version();
            if resources.atlas_version != atlas_version {
                let atlas_size = scene.font_sys.atlas_size();
                let atlas_pixels = scene.font_sys.atlas_pixels();
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &resources.atlas_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &atlas_pixels,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(atlas_size[0] as u32),
                        rows_per_image: Some(atlas_size[1] as u32),
                    },
                    wgpu::Extent3d {
                        width: atlas_size[0] as u32,
                        height: atlas_size[1] as u32,
                        depth_or_array_layers: 1,
                    },
                );
                resources.atlas_version = atlas_version;
            }
            while resources.surfaces.len() < scene.surface_caches.len() {
                let surface_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("surface"),
                            contents: bytemuck::cast_slice(&[0.0_f32, 0.0, 0.0, 0.0]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });
                let surface_bind_group =
                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("surface bind group"),
                        layout: &resources.surface_bind_group_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: surface_buffer.as_entire_binding(),
                        }],
                    });
                resources.surfaces.push(SurfaceResources {
                    surface_bind_group,
                    surface_buffer,
                    vertex_buffer: self.device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("surface vertices"),
                            contents: &[],
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        },
                    ),
                    content_version: 0,
                    vertex_capacity: 0,
                });
            }
            resources.surfaces.truncate(scene.surface_caches.len());
            for (index, surface) in scene.surface_caches.iter().enumerate() {
                let resources = &mut resources.surfaces[index];
                if resources.content_version == surface.content_version {
                    continue;
                }
                if surface.vertices.len() > resources.vertex_capacity {
                    resources.vertex_buffer =
                        self.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("surface vertices"),
                                contents: bytemuck::cast_slice(&surface.vertices),
                                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            });
                    resources.vertex_capacity = surface.vertices.len();
                } else {
                    self.queue.write_buffer(
                        &resources.vertex_buffer,
                        0,
                        bytemuck::cast_slice(&surface.vertices),
                    );
                }
                resources.content_version = surface.content_version;
            }
            for surface in scene.surfaces {
                self.queue.write_buffer(
                    &resources.surfaces[surface.cache_index].surface_buffer,
                    0,
                    bytemuck::cast_slice(&[
                        surface.origin[0],
                        surface.origin[1],
                        surface.origin[2],
                        0.0,
                    ]),
                );
            }
        }

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: scene.background[0] as f64,
                            g: scene.background[1] as f64,
                            b: scene.background[2] as f64,
                            a: scene.background[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(resources) = &self.resources {
                pass.set_pipeline(&resources.render_pipeline);
                pass.set_bind_group(0, &resources.camera_bind_group, &[]);
                pass.set_bind_group(1, &resources.atlas_bind_group, &[]);
                for surface in scene.surfaces {
                    let resource = &resources.surfaces[surface.cache_index];
                    let vertex_count =
                        scene.surface_caches[surface.cache_index].vertices.len() as u32;
                    pass.set_bind_group(2, &resource.surface_bind_group, &[]);
                    pass.set_vertex_buffer(0, resource.vertex_buffer.slice(..));
                    pass.draw(0..vertex_count, 0..1);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        RenderStatus::Presented
    }
}

fn log_surface_config(label: &str, config: &wgpu::SurfaceConfiguration) {
    log::debug!(
        concat!(
            "surface {}: size={}x{}, format={:?}, present_mode={:?}, ",
            "alpha_mode={:?}, max_frame_latency={}"
        ),
        label,
        config.width,
        config.height,
        config.format,
        config.present_mode,
        config.alpha_mode,
        config.desired_maximum_frame_latency,
    );
}
