use {
    anyhow::Result,
    heven::{
        App, Frame, InputEvent, Options, PhysicalSize, Scene, Surface, Text, TextSpan, TextStyle,
        oklch, rgb, rgba,
    },
};

fn main() -> Result<()> {
    env_logger::init();

    let mut surface1 = Surface::new([-1.5, 0.7, 6.0]);
    {
        let mut frame = Frame::new([0.0, 0.0], [1.8, 1.8], rgba(80, 120, 180, 32));
        for (line_index, line) in include_str!("../README.md").lines().enumerate() {
            frame.add(Text::new(
                [0.0, line_index as f32 * 0.08],
                vec![TextSpan::new(line, TextStyle::new(rgb(0, 0, 0)))],
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
        use_sdf_text: true,
    });

    let mut surface2_x = 1.0;
    app.animate(move |scene, dt| {
        surface2_x -= dt * 0.01;
        scene.position_surface(1, [surface2_x, 0.6, 12.0]);
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
