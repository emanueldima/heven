#![deny(
	warnings,
	missing_debug_implementations,
	missing_copy_implementations,
	bare_trait_objects,
	// missing_docs
)]

pub use {
    base::app::{App, InputEvent, Options},
    scene::{
        Camera, Frame, LinearRGB, Scene, Surface, Text, TextBounds, TextSpan, TextStyle, hsl,
        max_chroma, oklch, rgb, rgba,
    },
    winit::dpi::PhysicalSize,
};

pub mod base;
pub(crate) mod render;
pub mod scene;

// The platform layer should have support for:
// - graphic outputs: text, shapes
// - input events: keyboard, mouse
// - system events: quit, signals
// - time tracking, timers
// - sound
