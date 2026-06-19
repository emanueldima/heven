mod atlas;
mod sdf;

use {
    crate::scene::{FONT_SIZE, Frame, LINE_HEIGHT, TEXT_SCALE, Text, TextBounds},
    atlas::GlyphAtlas,
    cosmic_text::{Attrs, Buffer, Color, FontSystem, LayoutGlyph, Metrics, Shaping, SwashCache},
    std::{
        cell::{Ref, RefCell},
        collections::HashMap,
        time::Duration,
    },
};

#[derive(Debug)]
pub(crate) struct FontSys {
    state: RefCell<FontState>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FontQuad {
    pub(crate) origin: [f32; 3],
    pub(crate) size: [f32; 2],
    pub(crate) color: [u8; 4],
    pub(crate) tex_coords: [[f32; 2]; 4],
}

#[derive(Debug)]
struct FontState {
    font_system: FontSystem,
    swash_cache: SwashCache,
    glyph_atlas: GlyphAtlas,
    shaping_cache: HashMap<usize, ShapedText>,
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

impl FontSys {
    pub(crate) const ATLAS_SIZE: usize = GlyphAtlas::SIZE;

    pub(crate) fn new(font_name: &str) -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().set_sans_serif_family(font_name);

        Self {
            state: RefCell::new(FontState {
                font_system,
                swash_cache: SwashCache::new(),
                glyph_atlas: GlyphAtlas::new(GlyphAtlas::SIZE, GlyphAtlas::SIZE),
                shaping_cache: HashMap::new(),
            }),
        }
    }

    pub(crate) fn frame_text_bounds(&self, frame: &Frame) -> Option<TextBounds> {
        let mut state = self.state.borrow_mut();
        let mut bounds: Option<TextBounds> = None;
        for text in &frame.texts {
            let Some(text_bounds) = text_bounds(&mut state.font_system, text) else {
                continue;
            };
            bounds = Some(match bounds {
                Some(bounds) => bounds.union(text_bounds),
                None => text_bounds,
            });
        }
        bounds
    }

    pub(crate) fn text_quads(
        &self,
        text: &Text,
        origin: [f32; 3],
        shaping_time: &mut Duration,
    ) -> Vec<FontQuad> {
        let mut state = self.state.borrow_mut();
        let glyphs = state.shaped_text(text, shaping_time).glyphs.clone();
        let mut quads = Vec::with_capacity(glyphs.len());
        for shaped_glyph in glyphs {
            let glyph_scale = GlyphAtlas::SDF_SCALE as f32;
            let physical = shaped_glyph
                .glyph
                .physical((0.0, shaped_glyph.line_y * glyph_scale), glyph_scale);
            let atlas_entry = {
                let FontState {
                    font_system,
                    swash_cache,
                    glyph_atlas,
                    ..
                } = &mut *state;
                glyph_atlas.get_or_insert(font_system, swash_cache, physical.cache_key)
            };
            let Some(atlas_entry) = atlas_entry else {
                continue;
            };
            let width = atlas_entry.size[0] as f32 * TEXT_SCALE;
            let height = atlas_entry.size[1] as f32 * TEXT_SCALE;
            let x = physical.x as f32 / glyph_scale + atlas_entry.offset[0];
            let y = physical.y as f32 / glyph_scale + atlas_entry.offset[1];
            let color = shaped_glyph
                .glyph
                .color_opt
                .unwrap_or(Color::rgb(255, 255, 255));
            quads.push(FontQuad {
                origin: [
                    origin[0] + x * TEXT_SCALE,
                    origin[1] - y * TEXT_SCALE,
                    origin[2],
                ],
                size: [width, height],
                color: [color.r(), color.g(), color.b(), color.a()],
                tex_coords: state.glyph_atlas.tex_coords(atlas_entry),
            });
        }
        quads
    }

    pub(crate) fn atlas_pixels(&self) -> Ref<'_, [u8]> {
        Ref::map(self.state.borrow(), |state| state.glyph_atlas.pixels())
    }

    pub(crate) fn atlas_size(&self) -> [usize; 2] {
        self.state.borrow().glyph_atlas.size()
    }

    pub(crate) fn atlas_version(&self) -> u64 {
        self.state.borrow().glyph_atlas.version()
    }

    pub(crate) fn solid_tex_coords(&self) -> [[f32; 2]; 4] {
        self.state.borrow().glyph_atlas.solid_tex_coords()
    }
}

impl FontState {
    fn shaped_text(&mut self, text: &Text, shaping_time: &mut Duration) -> &ShapedText {
        let key = text as *const Text as usize;
        let signature = text_signature(text);
        let cached = self
            .shaping_cache
            .get(&key)
            .is_some_and(|shaped_text| shaped_text.signature == signature);
        if !cached {
            let shaping_start = std::time::Instant::now();
            let shaped_text = shape_text(&mut self.font_system, text, signature);
            self.shaping_cache.insert(key, shaped_text);
            *shaping_time += shaping_start.elapsed();
        }
        &self.shaping_cache[&key]
    }
}

pub(crate) const TEXT_METRICS: Metrics = Metrics::new(FONT_SIZE, LINE_HEIGHT);

fn text_bounds(font_system: &mut FontSystem, text: &Text) -> Option<TextBounds> {
    let attrs = Attrs::new();
    let spans = text
        .spans
        .iter()
        .map(|span| (span.content.as_str(), attrs.clone()));
    let mut buffer = Buffer::new(font_system, TEXT_METRICS);
    buffer.set_rich_text(spans, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);

    let mut bounds: Option<TextBounds> = None;
    for run in buffer.layout_runs() {
        let run_bounds = TextBounds {
            origin: [text.start[0], text.start[1] + run.line_top * TEXT_SCALE],
            size: [run.line_w * TEXT_SCALE, run.line_height * TEXT_SCALE],
        };
        bounds = Some(match bounds {
            Some(bounds) => bounds.union(run_bounds),
            None => run_bounds,
        });
    }
    bounds
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
