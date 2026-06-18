use {
    crate::scene::{FONT_SIZE, Frame, LINE_HEIGHT, TEXT_SCALE, Text, TextBounds},
    cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping},
};

pub(crate) fn frame_text_bounds(font_system: &mut FontSystem, frame: &Frame) -> Option<TextBounds> {
    let mut bounds: Option<TextBounds> = None;
    for text in &frame.texts {
        let Some(text_bounds) = text_bounds(font_system, text) else {
            continue;
        };
        bounds = Some(match bounds {
            Some(bounds) => bounds.union(text_bounds),
            None => text_bounds,
        });
    }
    bounds
}

fn text_bounds(font_system: &mut FontSystem, text: &Text) -> Option<TextBounds> {
    let attrs = Attrs::new();
    let spans = text
        .spans
        .iter()
        .map(|span| (span.content.as_str(), attrs.clone()));
    let mut buffer = Buffer::new(font_system, Metrics::new(FONT_SIZE, LINE_HEIGHT));
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
