use {
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
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AtlasEntry {
    pub(crate) origin: [usize; 2],
    pub(crate) size: [usize; 2],
    pub(crate) offset: [i32; 2],
}

impl GlyphAtlas {
    pub(crate) const SIZE: usize = 1024;

    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![0; width * height],
            width,
            height,
            entries: HashMap::new(),
            cursor: [0, 0],
            row_height: 0,
            version: 1,
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

        let size = [
            image.placement.width as usize,
            image.placement.height as usize,
        ];
        let offset = [image.placement.left, -image.placement.top];
        let origin = self.reserve(size)?;
        for y in 0..size[1] {
            let source = y * size[0];
            let target = (origin[1] + y) * self.width + origin[0];
            self.pixels[target..target + size[0]]
                .copy_from_slice(&image.data[source..source + size[0]]);
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
