use super::{LinearRGB, Text, rgba};

#[derive(Clone, Debug)]
pub struct Frame {
    pub(crate) origin: [f32; 2],
    pub(crate) size: [f32; 2],
    pub(crate) background: LinearRGB,
    pub(crate) elements: Vec<Text>,
}

impl Frame {
    pub fn new(origin: [f32; 2], size: [f32; 2], background: LinearRGB) -> Self {
        Self {
            origin,
            size,
            background,
            elements: Vec::new(),
        }
    }

    pub fn add(&mut self, text: Text) -> &mut Self {
        self.elements.push(text);
        self
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new([0.0, 0.0], [1.0, 1.0], rgba(0, 0, 0, 0))
    }
}
