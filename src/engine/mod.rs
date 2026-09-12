#[cfg(target_arch = "wasm32")]
pub mod canvas;
pub mod input;
pub mod math;
pub mod render;
pub mod world;

use crate::engine::input::{FrameInput, InputState};
use crate::engine::render::Renderer;
use crate::engine::world::World;

/// Everything a `System` may touch while running each tick.
pub struct TickContext<'a> {
    pub world: &'a mut World,
    pub input: &'a InputState,
    pub frame: &'a FrameInput,
    pub dt: f32,
}

/// A discrete gameplay step that reads from / writes to the `World`.
///
/// Systems are the extension point of this engine: add a new `System`
/// implementation and register it in `Game::new` to grow the game.
pub trait System {
    fn run(&mut self, ctx: &mut TickContext<'_>);
}

/// Convenience tuple for platforms that run one `Camera`-less fixed loop.
/// `simulate` advances the short-lived gameplay systems; `render` draws.
pub trait GameDriver {
    fn simulate(&mut self, dt: f32, input: &InputState, frame: &FrameInput);
    fn draw(&self, renderer: &mut dyn Renderer);
}