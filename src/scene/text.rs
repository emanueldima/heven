use super::LinearRGB;

#[derive(Clone, Debug)]
pub struct Text {
    pub(crate) start: [f32; 2],
    pub(crate) spans: Vec<TextSpan>,
}

pub const FONT_SIZE: f32 = 32.0;
pub const LINE_HEIGHT: f32 = 48.0;

impl Text {
    pub fn new(start: [f32; 2], spans: Vec<TextSpan>) -> Self {
        Self { start, spans }
    }

    pub fn span(&mut self, span: TextSpan) -> &mut Self {
        self.spans.push(span);
        self
    }
}

#[derive(Clone, Debug)]
pub struct TextSpan {
    pub(crate) content: String,
    pub(crate) style: TextStyle,
}

impl TextSpan {
    pub fn new(content: &str, style: TextStyle) -> Self {
        Self {
            content: content.to_owned(),
            style,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextStyle {
    pub(crate) color: LinearRGB,
}

impl TextStyle {
    pub fn new(color: LinearRGB) -> Self {
        Self { color }
    }
}
