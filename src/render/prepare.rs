use {
    super::{
        GlyphAtlas, Vertex,
        atlas::AtlasEntry,
        geometry::{self, push_quad},
    },
    crate::scene::{Camera, FONT_SIZE, LINE_HEIGHT, Scene, Text},
    cosmic_text::{Attrs, Buffer, Color, FontSystem, LayoutGlyph, Metrics, Shaping, SwashCache},
    std::{
        collections::{HashMap, hash_map::DefaultHasher},
        hash::{Hash, Hasher},
        time::{Duration, Instant},
    },
};

#[derive(Debug)]
pub(crate) struct SceneRenderCache {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) glyph_atlas: GlyphAtlas,
    pub(crate) font_system: FontSystem,
    pub(crate) swash_cache: SwashCache,
    shaping_cache: HashMap<usize, ShapedText>,
    pub(crate) content_version: u64,
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
    pub(crate) vertices: &'a [Vertex],
    pub(crate) glyph_atlas: &'a GlyphAtlas,
    pub(crate) camera: Camera,
    pub(crate) background: [f32; 4],
    pub(crate) content_version: u64,
    pub(crate) shaping_time: Duration,
}

impl SceneRenderCache {
    pub(crate) fn new(use_sdf_text: bool) -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().set_sans_serif_family("Helvetica Neue");

        Self {
            vertices: Vec::new(),
            glyph_atlas: GlyphAtlas::new(GlyphAtlas::SIZE, GlyphAtlas::SIZE, use_sdf_text),
            font_system,
            swash_cache: SwashCache::new(),
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
            vertices: &cache.vertices,
            glyph_atlas: &cache.glyph_atlas,
            camera: scene.camera,
            background: scene.background.as_floats(),
            content_version: cache.content_version,
            shaping_time: Duration::ZERO,
        };
    }

    let frame_count = scene
        .surfaces
        .iter()
        .map(|surface| surface.frames.len())
        .sum::<usize>();
    let rune_count = scene
        .surfaces
        .iter()
        .flat_map(|surface| &surface.frames)
        .flat_map(|frame| &frame.elements)
        .flat_map(|text| &text.spans)
        .map(|span| span.content.chars().count())
        .sum::<usize>();
    cache.vertices.clear();
    cache
        .vertices
        .reserve((frame_count + rune_count) * geometry::QUAD_VERTEX_COUNT);
    let mut shaping_time = Duration::ZERO;
    for surface in &scene.surfaces {
        for frame in &surface.frames {
            push_quad(
                &mut cache.vertices,
                [
                    surface.origin[0] + frame.origin[0],
                    surface.origin[1] - frame.origin[1],
                    surface.origin[2],
                ],
                frame.size,
                frame.background.as_bytes(),
                [[0.0, 0.0]; 4],
                0.0,
            );
            for text in &frame.elements {
                let shaped_text = shaped_text(
                    &mut cache.shaping_cache,
                    &mut cache.font_system,
                    text,
                    &mut shaping_time,
                );
                push_shaped_text_quads(
                    &mut cache.vertices,
                    &mut cache.font_system,
                    &mut cache.swash_cache,
                    &mut cache.glyph_atlas,
                    [
                        surface.origin[0] + frame.origin[0] + text.start[0],
                        surface.origin[1] - frame.origin[1] - text.start[1],
                        surface.origin[2],
                    ],
                    shaped_text,
                );
            }
        }
    }
    cache.content_version = scene.content_version;

    SceneRenderData {
        vertices: &cache.vertices,
        glyph_atlas: &cache.glyph_atlas,
        camera: scene.camera,
        background: scene.background.as_floats(),
        content_version: cache.content_version,
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
        let glyph_scale = if glyph_atlas.uses_sdf() {
            GlyphAtlas::SDF_SCALE as f32
        } else {
            1.0
        };
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
            if glyph_atlas.uses_sdf() { 2.0 } else { 1.0 },
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
    let mut hasher = DefaultHasher::new();
    for span in &text.spans {
        span.content.hash(&mut hasher);
        span.style.color.as_bytes().hash(&mut hasher);
    }
    hasher.finish()
}

fn atlas_tex_coords(glyph_atlas: &GlyphAtlas, entry: AtlasEntry) -> [[f32; 2]; 4] {
    let size = glyph_atlas.size();
    let left = entry.origin[0] as f32 / size[0] as f32;
    let top = entry.origin[1] as f32 / size[1] as f32;
    let right = (entry.origin[0] + entry.size[0]) as f32 / size[0] as f32;
    let bottom = (entry.origin[1] + entry.size[1]) as f32 / size[1] as f32;
    [[left, bottom], [right, bottom], [right, top], [left, top]]
}
