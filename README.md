# Heven

Heven is a small Rust UI platform layer built on `winit` and `wgpu`.

The goal is to use it as a library: callers create an app, build a scene, hand
that scene to the app, and then drive the program loop from their own example or
binary.

The first API target is text and tree-like content on 2D surfaces. Later, the
same shape should extend to organizing 3D surfaces.

## Todos

- vertice mode -> uniform?
- text wrapping?
- keyboard events
- tree of text panes, auto layout

## Running

```sh
cargo run --example full
```

## Architecture

The current top-level flow is:

```rust
let mut app = App::new(options);
let mut scene = Scene::new();

// Build surfaces, frames, and text.

app.render(scene);
app.run()
```

`App` owns the window, renderer, event loop integration, and the current scene.
The example creates the app and decides when to call `run`; the app does not own
the user's program structure before that point.

`Scene` owns the 2D content to render. It currently contains:

- a background color
- a camera
- positioned `Surface` values

`Surface` is positioned in scene space and contains frames.

`Frame` is positioned relative to its surface and contains text elements.

`Text` is positioned relative to its frame. A text value is a `Vec<TextSpan>`,
rendered inline, where each span has its own style.

`TextSpan` contains string content and a `TextStyle`.

`TextStyle` currently contains text color.

`LinearRGB` is the RGBA byte color type stored by the scene. The `rgb`, `rgba`,
`hsl`, and `oklch` helper functions create user-facing colors.

The transform stack for text is:

```text
surface origin + surface-local frame origin + frame-local text start + glyph layout position
```

The scene API tracks a content version for geometry-affecting changes. Camera
and background changes are frame state and do not invalidate prepared geometry.
`SceneRenderCache` lives in the `render` module. It owns cached vertices, text
layout state, and the glyph atlas, and turns scene content into `SceneRenderData`
for the renderer.

```rust
let scene_render_data = prepare_scene(&mut scene_render_cache, scene);
renderer.render(&scene_render_data);
```

For library users this is hidden behind `App`: the app rebuilds the scene before
rendering each redraw.

`Renderer` owns GPU resources and GPU upload synchronization. It tracks the
uploaded glyph atlas version and uploaded vertex content version, then uploads
only when the corresponding CPU-side version changes.

Text layout uses `cosmic-text`. Glyphs are rasterized into a CPU-side alpha
atlas, uploaded to the GPU, and drawn as textured quads. This is intentionally
small for now: it proves the layout and atlas path without committing to the
final font system.

Surface origins are placed in WebGPU clip-space-like scene coordinates:

- `x = -1.0` is left, `x = 1.0` is right
- `y = -1.0` is bottom, `y = 1.0` is top

Coordinates inside a surface are page-like:

- `x = 0.0` starts at the surface's left edge and positive `x` moves right
- `y = 0.0` starts at the surface's top edge and positive `y` moves down
- frames and text are positioned in this surface-local coordinate system

The current module layout is:

```text
src/
  base.rs
  base/
    app.rs
    renderer.rs
  render/
    atlas.rs
    geometry.rs
    prepare.rs
  scene/
    color.rs
    frame.rs
    mod.rs
    surface.rs
    text.rs
  shaders/
    scene.wgsl
```

`base` is the platform layer: app lifecycle, frame timing, window creation,
renderer setup, GPU resources, and redraw scheduling.

`scene` is the public content model: scene, surfaces, frames, text, spans, and
styles.

`render` is the crate-private preparation layer: it reads scene content, lays out
text, manages the glyph atlas and render cache, and builds renderer vertices.
