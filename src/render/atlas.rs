use {
    super::sdf,
    cosmic_text::{CacheKey, FontSystem, SwashCache, SwashContent},
    std::collections::HashMap,
};

#[derive(Clone, Debug)]
pub(crate) struct GlyphAtlas {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    entries: HashMap<CacheKey, AtlasEntry>,
    cursor: [usize; 2],
    row_height: usize,
    version: u64,
    use_sdf: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AtlasEntry {
    pub(crate) origin: [usize; 2],
    pub(crate) size: [usize; 2],
    pub(crate) offset: [f32; 2],
}

impl GlyphAtlas {
    pub(crate) const SIZE: usize = 1024;
    pub(crate) const SDF_SCALE: usize = 3;

    pub(crate) fn new(width: usize, height: usize, use_sdf: bool) -> Self {
        Self {
            pixels: vec![0; width * height],
            width,
            height,
            entries: HashMap::new(),
            cursor: [0, 0],
            row_height: 0,
            version: 1,
            use_sdf,
        }
    }

    pub(crate) fn get_or_insert(
        &mut self,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        cache_key: CacheKey,
    ) -> Option<AtlasEntry> {
        if let Some(entry) = self.entries.get(&cache_key) {
            return Some(*entry);
        }

        let image = swash_cache.get_image(font_system, cache_key).as_ref()?;
        if image.content != SwashContent::Mask {
            return None;
        }

        let source_size = [
            image.placement.width as usize,
            image.placement.height as usize,
        ];
        let scale = if self.use_sdf { Self::SDF_SCALE } else { 1 };
        let size = [
            source_size[0].div_ceil(scale) + GLYPH_PADDING * 2,
            source_size[1].div_ceil(scale) + GLYPH_PADDING * 2,
        ];
        let offset = [
            image.placement.left as f32 / scale as f32 - GLYPH_PADDING as f32,
            -image.placement.top as f32 / scale as f32 - GLYPH_PADDING as f32,
        ];
        let origin = self.reserve(size)?;
        let mask = padded_mask(&image.data, source_size, size, scale);
        let pixels = if self.use_sdf {
            downsample(
                &sdf::sdf(&mask, [size[0] * scale, size[1] * scale]),
                size,
                scale,
            )
        } else {
            mask
        };
        for y in 0..size[1] {
            let source = y * size[0];
            let target = (origin[1] + y) * self.width + origin[0];
            self.pixels[target..target + size[0]]
                .copy_from_slice(&pixels[source..source + size[0]]);
        }

        let entry = AtlasEntry {
            origin,
            size,
            offset,
        };
        self.entries.insert(cache_key, entry);
        self.version += 1;
        Some(entry)
    }

    pub(crate) fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub(crate) fn size(&self) -> [usize; 2] {
        [self.width, self.height]
    }

    pub(crate) fn version(&self) -> u64 {
        self.version
    }

    pub(crate) fn uses_sdf(&self) -> bool {
        self.use_sdf
    }

    fn reserve(&mut self, size: [usize; 2]) -> Option<[usize; 2]> {
        if size[0] == 0 || size[1] == 0 || size[0] > self.width || size[1] > self.height {
            return None;
        }
        if self.cursor[0] + size[0] > self.width {
            self.cursor[0] = 0;
            self.cursor[1] += self.row_height;
            self.row_height = 0;
        }
        if self.cursor[1] + size[1] > self.height {
            return None;
        }

        let origin = self.cursor;
        self.cursor[0] += size[0];
        self.row_height = self.row_height.max(size[1]);
        Some(origin)
    }
}

const GLYPH_PADDING: usize = 8;

fn padded_mask(source: &[u8], source_size: [usize; 2], size: [usize; 2], scale: usize) -> Vec<u8> {
    let mask_size = [size[0] * scale, size[1] * scale];
    let mut mask = vec![0; mask_size[0] * mask_size[1]];
    for y in 0..source_size[1] {
        let source_start = y * source_size[0];
        let target_start = (y + GLYPH_PADDING * scale) * mask_size[0] + GLYPH_PADDING * scale;
        mask[target_start..target_start + source_size[0]]
            .copy_from_slice(&source[source_start..source_start + source_size[0]]);
    }
    mask
}

fn downsample(source: &[u8], size: [usize; 2], scale: usize) -> Vec<u8> {
    let source_width = size[0] * scale;
    let mut pixels = vec![0; size[0] * size[1]];
    for y in 0..size[1] {
        for x in 0..size[0] {
            let mut sum = 0;
            for dy in 0..scale {
                for dx in 0..scale {
                    sum += source[(y * scale + dy) * source_width + x * scale + dx] as usize;
                }
            }
            pixels[y * size[0] + x] = (sum / (scale * scale)) as u8;
        }
    }
    pixels
}
