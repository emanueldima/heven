use {
    super::{
        GlyphAtlas, Vertex,
        atlas::AtlasEntry,
        geometry::{self, push_quad},
    },
    crate::scene::{Camera, FONT_SIZE, LINE_HEIGHT, Scene, Text},
    cosmic_text::{Attrs, Buffer, Color, FontSystem, LayoutGlyph, Metrics, Shaping, SwashCache},
    std::{
        collections::HashMap,
        time::{Duration, Instant},
    },
};

#[derive(Debug)]
pub(crate) struct SceneRenderCache {
    pub(crate) glyph_atlas: GlyphAtlas,
    pub(crate) font_system: FontSystem,
    pub(crate) swash_cache: SwashCache,
    surface_caches: Vec<SurfaceRenderCache>,
    surfaces: Vec<SceneRenderSurface>,
    shaping_cache: HashMap<usize, ShapedText>,
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

#[derive(Clone, Debug)]
struct ShapedText {
    signature: u64,
    glyphs: Vec<ShapedGlyph>,
}

#[derive(Clone, Debug)]
struct ShapedGlyph {
    line_y: f32,
    glyph: LayoutGlyph,
}

#[derive(Debug)]
pub(crate) struct SceneRenderData<'a> {
    pub(crate) surfaces: &'a [SceneRenderSurface],
    pub(crate) surface_caches: &'a [SurfaceRenderCache],
    pub(crate) glyph_atlas: &'a GlyphAtlas,
    pub(crate) camera: Camera,
    pub(crate) background: [f32; 4],
    pub(crate) shaping_time: Duration,
}

impl SceneRenderCache {
    pub(crate) fn new(font_name: &str) -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().set_sans_serif_family(font_name);

        Self {
            glyph_atlas: GlyphAtlas::new(GlyphAtlas::SIZE, GlyphAtlas::SIZE),
            font_system,
            swash_cache: SwashCache::new(),
            surface_caches: Vec::new(),
            surfaces: Vec::new(),
            shaping_cache: HashMap::new(),
            content_version: 0,
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
            glyph_atlas: &cache.glyph_atlas,
            camera: scene.camera,
            background: scene.background.as_floats(),
            shaping_time: Duration::ZERO,
        };
    }

    let t0 = Instant::now();
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
                cache.glyph_atlas.solid_tex_coords(),
            );
            for text in &frame.texts {
                let shaped_text = shaped_text(
                    &mut cache.shaping_cache,
                    &mut cache.font_system,
                    text,
                    &mut shaping_time,
                );
                push_shaped_text_quads(
                    &mut surface_cache.vertices,
                    &mut cache.font_system,
                    &mut cache.swash_cache,
                    &mut cache.glyph_atlas,
                    [
                        frame.origin[0] + text.start[0],
                        -frame.origin[1] - text.start[1],
                        0.0,
                    ],
                    shaped_text,
                );
            }
        }
        surface_cache.content_version = surface.content_version;
    }
    cache.content_version = scene.content_version;

    log::debug!("prepare scene: {:?}", t0.elapsed());
    SceneRenderData {
        surfaces: &cache.surfaces,
        surface_caches: &cache.surface_caches,
        glyph_atlas: &cache.glyph_atlas,
        camera: scene.camera,
        background: scene.background.as_floats(),
        shaping_time,
    }
}

const TEXT_SCALE: f32 = 0.0025;
pub const TEXT_METRICS: Metrics = Metrics::new(FONT_SIZE, LINE_HEIGHT);

fn push_shaped_text_quads(
    vertices: &mut Vec<Vertex>,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_atlas: &mut GlyphAtlas,
    origin: [f32; 3],
    text: &ShapedText,
) {
    for shaped_glyph in &text.glyphs {
        let glyph_scale = GlyphAtlas::SDF_SCALE as f32;
        let physical = shaped_glyph
            .glyph
            .physical((0.0, shaped_glyph.line_y * glyph_scale), glyph_scale);
        let atlas_entry = glyph_atlas.get_or_insert(font_system, swash_cache, physical.cache_key);
        let Some(atlas_entry) = atlas_entry else {
            continue;
        };
        let width = atlas_entry.size[0] as f32 * TEXT_SCALE;
        let height = atlas_entry.size[1] as f32 * TEXT_SCALE;
        let x = physical.x as f32 / glyph_scale + atlas_entry.offset[0];
        let y = physical.y as f32 / glyph_scale + atlas_entry.offset[1];
        push_quad(
            vertices,
            [
                origin[0] + x * TEXT_SCALE,
                origin[1] - y * TEXT_SCALE,
                origin[2],
            ],
            [width, height],
            text_color(
                shaped_glyph
                    .glyph
                    .color_opt
                    .unwrap_or(Color::rgb(255, 255, 255)),
            ),
            atlas_tex_coords(glyph_atlas, atlas_entry),
        );
    }
}

fn shaped_text<'a>(
    shaping_cache: &'a mut HashMap<usize, ShapedText>,
    font_system: &mut FontSystem,
    text: &Text,
    shaping_time: &mut Duration,
) -> &'a ShapedText {
    let key = text as *const Text as usize;
    let signature = text_signature(text);
    let cached = shaping_cache
        .get(&key)
        .is_some_and(|shaped_text| shaped_text.signature == signature);
    if !cached {
        let shaping_start = Instant::now();
        shaping_cache.insert(key, shape_text(font_system, text, signature));
        *shaping_time += shaping_start.elapsed();
    }
    &shaping_cache[&key]
}

fn shape_text(font_system: &mut FontSystem, text: &Text, signature: u64) -> ShapedText {
    log::debug!("shaping text, {} spans", text.spans.len());
    let attrs = Attrs::new();
    let spans = text.spans.iter().map(|span| {
        let [red, green, blue, alpha] = span.style.color.as_bytes();
        (
            span.content.as_str(),
            attrs.clone().color(Color::rgba(red, green, blue, alpha)),
        )
    });
    let mut buffer = Buffer::new(font_system, TEXT_METRICS);
    buffer.set_rich_text(spans, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);

    let mut glyphs = Vec::new();
    for run in buffer.layout_runs() {
        glyphs.extend(run.glyphs.iter().cloned().map(|glyph| ShapedGlyph {
            line_y: run.line_y,
            glyph,
        }));
    }
    ShapedText { signature, glyphs }
}

fn text_color(color: Color) -> [u8; 4] {
    [color.r(), color.g(), color.b(), color.a()]
}

fn text_signature(text: &Text) -> u64 {
    let mut hash = text.spans.len() as u64;
    for span in &text.spans {
        hash = hash
            .wrapping_mul(31)
            .wrapping_add(span.content.len() as u64);
        for byte in span.content.as_bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        for byte in span.style.color.as_bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
    }
    hash
}

fn atlas_tex_coords(glyph_atlas: &GlyphAtlas, entry: AtlasEntry) -> [[f32; 2]; 4] {
    let size = glyph_atlas.size();
    let left = entry.origin[0] as f32 / size[0] as f32;
    let top = entry.origin[1] as f32 / size[1] as f32;
    let right = (entry.origin[0] + entry.size[0]) as f32 / size[0] as f32;
    let bottom = (entry.origin[1] + entry.size[1]) as f32 / size[1] as f32;
    [[left, bottom], [right, bottom], [right, top], [left, top]]
}
