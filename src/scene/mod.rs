mod camera;
mod color;
mod frame;
mod surface;
mod text;

pub use {
    camera::Camera,
    color::{LinearRGB, hsl, max_chroma, oklch, rgb, rgba},
    frame::Frame,
    surface::Surface,
    text::{FONT_SIZE, LINE_HEIGHT, Text, TextSpan, TextStyle},
};

#[derive(Clone, Debug)]
pub struct Scene {
    pub camera: Camera,
    pub(crate) background: LinearRGB,
    pub(crate) surfaces: Vec<Surface>,
    pub(crate) content_version: u64,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            background: rgb(128, 128, 128),
            surfaces: Vec::new(),
            content_version: 1,
        }
    }

    pub fn background(&mut self, color: LinearRGB) -> &mut Self {
        self.background = color;
        self
    }

    pub fn add(&mut self, surface: Surface) -> &mut Self {
        self.surfaces.push(surface);
        self.content_version += 1;
        self
    }

    pub fn surface_position_mut(&mut self, index: usize) -> Option<&mut [f32; 3]> {
        self.content_version += 1;
        if let Some(surface) = self.surfaces.get_mut(index) {
            Some(&mut surface.origin)
        } else {
            None
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
