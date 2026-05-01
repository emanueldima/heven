use {
    super::{
        GlyphAtlas, Vertex,
        atlas::AtlasEntry,
        geometry::{self, push_quad},
    },
    crate::scene::{Camera, FONT_SIZE, LINE_HEIGHT, Scene, Text},
    cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache},
};

#[derive(Debug)]
pub(crate) struct SceneRenderCache {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) glyph_atlas: GlyphAtlas,
    pub(crate) font_system: FontSystem,
    pub(crate) swash_cache: SwashCache,
    pub(crate) content_version: u64,
}

#[derive(Debug)]
pub(crate) struct SceneRenderData<'a> {
    pub(crate) vertices: &'a [Vertex],
    pub(crate) glyph_atlas: &'a GlyphAtlas,
    pub(crate) camera: Camera,
    pub(crate) background: [f32; 4],
    pub(crate) content_version: u64,
}

impl SceneRenderCache {
    pub(crate) fn new() -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().set_sans_serif_family("Helvetica Neue");

        Self {
            vertices: Vec::new(),
            glyph_atlas: GlyphAtlas::new(GlyphAtlas::SIZE, GlyphAtlas::SIZE),
            font_system,
            swash_cache: SwashCache::new(),
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
                push_text_quads(
                    &mut cache.vertices,
                    &mut cache.font_system,
                    &mut cache.swash_cache,
                    &mut cache.glyph_atlas,
                    [
                        surface.origin[0] + frame.origin[0] + text.start[0],
                        surface.origin[1] - frame.origin[1] - text.start[1],
                        surface.origin[2],
                    ],
                    text,
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
    }
}

const TEXT_SCALE: f32 = 0.0025;
pub const TEXT_METRICS: Metrics = Metrics::new(FONT_SIZE, LINE_HEIGHT);

fn push_text_quads(
    vertices: &mut Vec<Vertex>,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_atlas: &mut GlyphAtlas,
    origin: [f32; 3],
    text: &Text,
) {
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

    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, run.line_y), 1.0);
            let atlas_entry =
                glyph_atlas.get_or_insert(font_system, swash_cache, physical.cache_key);
            let Some(atlas_entry) = atlas_entry else {
                continue;
            };
            let width = atlas_entry.size[0] as f32 * TEXT_SCALE;
            let height = atlas_entry.size[1] as f32 * TEXT_SCALE;
            let x = physical.x + atlas_entry.offset[0];
            let y = physical.y + atlas_entry.offset[1];
            push_quad(
                vertices,
                [
                    origin[0] + x as f32 * TEXT_SCALE,
                    origin[1] - y as f32 * TEXT_SCALE,
                    origin[2],
                ],
                [width, height],
                text_color(glyph.color_opt.unwrap_or(Color::rgb(255, 255, 255))),
                atlas_tex_coords(glyph_atlas, atlas_entry),
                1.0,
            );
        }
    }
}

fn text_color(color: Color) -> [u8; 4] {
    [color.r(), color.g(), color.b(), color.a()]
}

fn atlas_tex_coords(glyph_atlas: &GlyphAtlas, entry: AtlasEntry) -> [[f32; 2]; 4] {
    let size = glyph_atlas.size();
    let left = entry.origin[0] as f32 / size[0] as f32;
    let top = entry.origin[1] as f32 / size[1] as f32;
    let right = (entry.origin[0] + entry.size[0]) as f32 / size[0] as f32;
    let bottom = (entry.origin[1] + entry.size[1]) as f32 / size[1] as f32;
    [[left, bottom], [right, bottom], [right, top], [left, top]]
}
