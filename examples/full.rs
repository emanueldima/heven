use {
    anyhow::Result,
    heven::{
        App, Frame, Options, PhysicalSize, Scene, Surface, Text, TextSpan, TextStyle, oklch, rgb,
        rgba,
    },
};

fn main() -> Result<()> {
    env_logger::init();

    let mut surface1 = Surface::new([-1.0, 0.7, 6.0]);
    {
        let mut frame = Frame::new([0.0, 0.0], [1.8, 1.8], rgba(80, 120, 180, 32));
        for (line_index, line) in include_str!("../README.md").lines().enumerate() {
            frame.add(Text::new(
                [0.0, line_index as f32 * 0.08],
                vec![TextSpan::new(line, TextStyle::new(rgb(20, 30, 40)))],
            ));
        }
        surface1.add(frame);
    }

    let mut surface2 = Surface::new([1.0, 0.6, 12.0]);
    {
        let mut frame = Frame::new([0.0, 0.0], [0.6, 0.32], rgba(240, 180, 60, 48));
        frame.add(Text::new(
            [0.0, 0.0],
            vec![
                TextSpan::new("Hello", TextStyle::new(rgb(200, 0, 0))),
                TextSpan::new(" inline", TextStyle::new(rgb(0, 20, 40))),
            ],
        ));
        frame.add(Text::new(
            [0.0, 0.16],
            vec![TextSpan::new("World", TextStyle::new(rgb(0, 50, 0)))],
        ));
        surface2.add(frame);

        let mut frame = Frame::new([-0.05, 0.45], [1.05, 0.72], rgba(0, 0, 0, 0));
        let text = "********************************";
        let mut spans = Vec::new();
        for row in 0..8 {
            let lightness = 0.25 + row as f32 / 7.0 * 0.65;
            if row > 0 {
                spans.push(TextSpan::new("\n", TextStyle::new(rgb(0, 0, 0))));
            }
            for (column, character) in text.chars().enumerate() {
                let hue = column as f32 / text.len() as f32 * 360.0;
                spans.push(TextSpan::new(
                    &character.to_string(),
                    TextStyle::new(oklch(lightness, 0.37, hue)),
                ));
            }
        }
        frame.add(Text::new([0.0, 0.0], spans));
        surface2.add(frame);
    }

    let mut scene = Scene::new();
    scene
        .background(rgb(255, 253, 245))
        .add(surface1)
        .add(surface2);

    let mut app = App::new(Options {
        title: "Heven full example",
        size: PhysicalSize::new(1600, 900),
    });

    app.animate(move |scene, dt| {
        scene.camera.position([
            scene.camera.x() + dt * 0.01,
            scene.camera.y(),
            (scene.camera.z() - dt * 1.0).max(20.0),
        ]);
    });

    app.render(scene);
    app.run()
}
