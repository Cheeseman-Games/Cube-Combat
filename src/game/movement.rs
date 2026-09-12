use crate::engine::world::EntityId;
use crate::engine::{System, TickContext};
use crate::game::constants::*;
use crate::game::fighters::{FighterState, Player, Transform};

/// Moves fighters, applies knockback decay and clamps to the arena, all from
/// immutable reads plus a two-pass write-back through one reused buffer.
#[derive(Default)]
pub struct MovementSystem {
    steps: Vec<(EntityId, f32, f32)>,
}

impl System for MovementSystem {
    fn run(&mut self, ctx: &mut TickContext<'_>) {
        let dt = ctx.dt;
        let (players, transforms) = (ctx.world.store::<Player>(), ctx.world.store::<Transform>());
        let (Some(players), Some(transforms)) = (players, transforms) else {
            return;
        };

        self.steps.clear();
        let n = players.len().min(transforms.len());
        for i in 0..n {
            let Some(player) = players[i].as_ref() else { continue };
            let Some(body) = transforms[i].as_ref() else { continue };

            let speed = match player.state {
                FighterState::Neutral => MOVE_SPEED,
                FighterState::Blocking => MOVE_SPEED * BLOCK_MOVE_FACTOR,
                _ => 0.0,
            };
            let vx = player.move_dir * speed + player.knock_vel;
            let new_x = (body.x + vx * dt).clamp(0.0, ARENA_W - body.w);
            let new_knock = player.knock_vel * KNOCK_DECAY;
            self.steps.push((i as EntityId, new_x, new_knock));
        }

        if !self.steps.is_empty() {
            let transforms = ctx.world.store_mut::<Transform>();
            for &(id, x, _) in &self.steps {
                if let Some(body) = transforms.get_mut(id as usize).and_then(Option::as_mut) {
                    body.x = x;
                }
            }
            let players = ctx.world.store_mut::<Player>();
            for &(id, _, knock) in &self.steps {
                if let Some(player) = players.get_mut(id as usize).and_then(Option::as_mut) {
                    player.knock_vel = knock;
                }
            }
        }
    }
}