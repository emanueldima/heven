use super::Frame;

#[derive(Clone, Debug)]
pub struct Surface {
    pub(crate) origin: [f32; 3],
    pub(crate) frames: Vec<Frame>,
}

impl Surface {
    pub fn new(origin: [f32; 3]) -> Self {
        Self {
            origin,
            frames: Vec::new(),
        }
    }

    pub fn add(&mut self, frame: Frame) -> &mut Self {
        self.frames.push(frame);
        self
    }
}

impl Default for Surface {
    fn default() -> Self {
        Self::new([0.0, 0.0, 0.0])
    }
}
