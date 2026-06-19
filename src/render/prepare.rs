use {
    super::{
        Vertex,
        geometry::{self, push_quad},
    },
    crate::{
        font::FontSys,
        scene::{Camera, Scene},
    },
    std::{
        rc::Rc,
        time::{Duration, Instant},
    },
};

#[derive(Debug)]
pub(crate) struct SceneRenderCache {
    pub(crate) font_sys: Rc<FontSys>,
    surface_caches: Vec<SurfaceRenderCache>,
    surfaces: Vec<SceneRenderSurface>,
    pub(crate) content_version: u64,
}

#[derive(Debug, Default)]
pub(crate) struct SurfaceRenderCache {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) content_version: u64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SceneRenderSurface {
    pub(crate) origin: [f32; 3],
    pub(crate) cache_index: usize,
}

#[derive(Debug)]
pub(crate) struct SceneRenderData<'a> {
    pub(crate) surfaces: &'a [SceneRenderSurface],
    pub(crate) surface_caches: &'a [SurfaceRenderCache],
    pub(crate) font_sys: Rc<FontSys>,
    pub(crate) camera: Camera,
    pub(crate) background: [f32; 4],
    pub(crate) shaping_time: Duration,
}

impl SceneRenderCache {
    pub(crate) fn new(font_sys: Rc<FontSys>) -> Self {
        Self {
            font_sys,
            surface_caches: Vec::new(),
            surfaces: Vec::new(),
            content_version: 0,
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.surfaces.clear();
        self.content_version = 0;
        for surface_cache in &mut self.surface_caches {
            surface_cache.content_version = 0;
        }
    }
}

pub(crate) fn prepare_scene<'a>(
    cache: &'a mut SceneRenderCache,
    scene: &Scene,
) -> SceneRenderData<'a> {
    if cache.content_version == scene.content_version {
        return SceneRenderData {
            surfaces: &cache.surfaces,
            surface_caches: &cache.surface_caches,
            font_sys: Rc::clone(&cache.font_sys),
            camera: scene.camera,
            background: scene.background.as_floats(),
            shaping_time: Duration::ZERO,
        };
    }

    let _t0 = Instant::now();
    cache.surfaces.clear();
    cache
        .surface_caches
        .resize_with(scene.surfaces.len(), Default::default);
    cache.surfaces.reserve(scene.surfaces.len());
    let mut shaping_time = Duration::ZERO;
    let mut surface_indices = (0..scene.surfaces.len()).collect::<Vec<_>>();
    surface_indices.sort_by(|left, right| {
        scene.surfaces[*left].origin[2].total_cmp(&scene.surfaces[*right].origin[2])
    });
    for surface_index in surface_indices {
        let surface = &scene.surfaces[surface_index];
        cache.surfaces.push(SceneRenderSurface {
            origin: surface.origin,
            cache_index: surface_index,
        });
        if cache.surface_caches[surface_index].content_version == surface.content_version {
            continue;
        }

        let surface_cache = &mut cache.surface_caches[surface_index];
        surface_cache.vertices.clear();
        surface_cache.vertices.reserve(
            surface
                .frames
                .iter()
                .map(|frame| {
                    1 + frame
                        .texts
                        .iter()
                        .flat_map(|text| &text.spans)
                        .map(|span| span.content.chars().count())
                        .sum::<usize>()
                })
                .sum::<usize>()
                * geometry::QUAD_VERTEX_COUNT,
        );
        for frame in &surface.frames {
            push_quad(
                &mut surface_cache.vertices,
                [frame.origin[0], -frame.origin[1], 0.0],
                frame.size,
                frame.background.as_bytes(),
                cache.font_sys.solid_tex_coords(),
            );
            for text in &frame.texts {
                for quad in cache.font_sys.text_quads(
                    text,
                    [
                        frame.origin[0] + text.start[0],
                        -frame.origin[1] - text.start[1],
                        0.0,
                    ],
                    &mut shaping_time,
                ) {
                    push_quad(
                        &mut surface_cache.vertices,
                        quad.origin,
                        quad.size,
                        quad.color,
                        quad.tex_coords,
                    );
                }
            }
        }
        surface_cache.content_version = surface.content_version;
    }
    cache.content_version = scene.content_version;

    // log::debug!("prepare scene: {:?}", _t0.elapsed());
    SceneRenderData {
        surfaces: &cache.surfaces,
        surface_caches: &cache.surface_caches,
        font_sys: Rc::clone(&cache.font_sys),
        camera: scene.camera,
        background: scene.background.as_floats(),
        shaping_time,
    }
}
