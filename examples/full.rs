use {
    anyhow::Result,
    heven::{
        App, Frame, InputEvent, Options, PhysicalSize, Scene, Surface, Text, TextSpan, TextStyle,
        max_chroma, oklch, rgb, rgba,
    },
};

const TEXT_MARGIN: f32 = 0.08;

fn main() -> Result<()> {
    env_logger::init();

    let mut app = App::new(Options {
        title: "Heven full example",
        size: PhysicalSize::new(1600, 900),
        // font_name: "Monaco",
        font_name: "Helvetica Neue",
    });
    let mut scene = Scene::new();
    scene.background(rgb(255, 253, 245));

    {
        let mut surface = Surface::new([-1.5, 0.7, 8.0]);
        let mut frame = Frame::new([0.0, 0.0], [0.0, 0.0], rgba(80, 120, 180, 32));
        for (line_index, line) in include_str!("../README.md").lines().enumerate() {
            frame.add(Text::new(
                [TEXT_MARGIN, TEXT_MARGIN + line_index as f32 * 0.08],
                vec![TextSpan::new(line, TextStyle::new(rgb(0, 0, 0)))],
            ));
        }
        if let Some(bounds) = app.text_bounds(&frame) {
            frame.size([
                bounds.origin[0] + bounds.size[0] + TEXT_MARGIN,
                bounds.origin[1] + bounds.size[1] + TEXT_MARGIN,
            ]);
        }
        surface.add(frame);
        scene.add(surface);
    }

    {
        let mut surface = Surface::new([1.0, 0.6, 9.0]);
        let mut frame = Frame::new([0.0, 0.0], [0.0, 0.0], rgba(240, 180, 60, 48));
        frame.add(Text::new(
            [TEXT_MARGIN, TEXT_MARGIN],
            vec![
                TextSpan::new("Hello", TextStyle::new(rgb(200, 0, 0))),
                TextSpan::new(" inline", TextStyle::new(rgb(0, 20, 40))),
            ],
        ));
        frame.add(Text::new(
            [TEXT_MARGIN, TEXT_MARGIN + 0.16],
            vec![TextSpan::new("World", TextStyle::new(rgb(0, 50, 0)))],
        ));
        if let Some(bounds) = app.text_bounds(&frame) {
            frame.size([
                bounds.origin[0] + bounds.size[0] + TEXT_MARGIN,
                bounds.origin[1] + bounds.size[1] + TEXT_MARGIN,
            ]);
        }
        surface.add(frame);
        scene.add(surface);
    }

    for surface_index in 0..10 {
        let mut surface = Surface::new([0.0, 0.0, 2.0 + surface_index as f32 / 5.0]);
        let mut frame = Frame::new([0.0, 0.0], [0.0, 0.0], rgba(0, 0, 0, 0));
        let text = "•".repeat(32);
        let mut spans = Vec::new();
        let lightness = 0.1 + surface_index as f32 * 0.09; // [0.1, 0.91]
        for row in 0..10 {
            if row > 0 {
                spans.push(TextSpan::new("\n", TextStyle::new(rgb(0, 0, 0))));
            }
            let chroma = 0.04 + row as f32 * 0.02;
            for (column, character) in text.chars().enumerate() {
                let hue = column as f32 / text.chars().count() as f32 * 360.0;
                let max_c = max_chroma(lightness, hue);
                log::debug!(
                    "hue {:.1}, lightness {:.1}, chroma {:.1}, max_chroma: {:.1}",
                    hue,
                    lightness,
                    chroma,
                    max_c
                );
                let c = if chroma > max_c { 0.0 } else { chroma };
                let l = if chroma > max_c { 0.9 } else { lightness };
                spans.push(TextSpan::new(
                    &character.to_string(),
                    TextStyle::new(oklch(l, c, hue)),
                ));
            }
        }
        frame.add(Text::new([TEXT_MARGIN, TEXT_MARGIN], spans));
        if let Some(bounds) = app.text_bounds(&frame) {
            frame.size([
                bounds.origin[0] + bounds.size[0] + TEXT_MARGIN,
                bounds.origin[1] + bounds.size[1] + TEXT_MARGIN,
            ]);
        }
        surface.add(frame);
        scene.add(surface);
    }

    app.animate(move |scene, dt| {
        scene.surface_position_mut(1).unwrap()[2] -= dt * 0.02;
    });

    app.input(|scene, event| match event {
        InputEvent::MouseWheel { delta, command } => {
            if command {
                let zoom = (-delta[1] * 0.1).exp();
                let z = (scene.camera.z() * zoom).clamp(5.0, 30.0);
                scene
                    .camera
                    .position([scene.camera.x(), scene.camera.y(), z]);
                return;
            }

            let pan = scene.camera.z() * 0.02;
            scene.camera.position([
                scene.camera.x() - delta[0] * pan,
                scene.camera.y() + delta[1] * pan,
                scene.camera.z(),
            ]);
        }
    });

    app.render(scene);
    app.run()
}
