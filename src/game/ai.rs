use crate::engine::{System, TickContext};
use crate::game::constants::*;
use crate::game::fighters::{FighterState, Player, Transform};
use crate::game::Side;

/// Difficulty profile for the 1-player AI.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AiDifficulty {
    Easy,
    Normal,
    Hard,
}

impl AiDifficulty {
    pub fn label(self) -> &'static str {
        match self {
            AiDifficulty::Easy => "Easy",
            AiDifficulty::Normal => "Normal",
            AiDifficulty::Hard => "Hard",
        }
    }

    /// Cycle to the previous / next difficulty.
    pub fn next(self, dir: i32) -> Self {
        let all = [AiDifficulty::Easy, AiDifficulty::Normal, AiDifficulty::Hard];
        let idx = (all.iter().position(|d| *d == self).unwrap() as i32 + dir.signum())
            .rem_euclid(3) as usize;
        all[idx]
    }
}

/// Drives the red cube in 1-player mode. On a short, difficulty-scaled
/// interval the AI picks a course of action: approach while far away, and when
/// in range either slash, hold block for a moment, or back off. Higher
/// difficulties decide more often and go for the attack more aggressively.
#[derive(Clone)]
pub struct AiControlSystem {
    difficulty: AiDifficulty,
    rng: u32,
    decide_at: f32,
    dir: f32,
    slash: bool,
    block: bool,
}

impl AiControlSystem {
    pub fn new(difficulty: AiDifficulty) -> Self {
        // `SystemTime` is unsupported on wasm32 (it panics), so use a constant
        // seed. The LCG still advances every roll, so the AI behavior stays
        // varied in practice while remaining deterministic for tests.
        let seed = 0x9e37_79b9 | 1;
        Self {
            difficulty,
            rng: seed,
            decide_at: 0.0,
            dir: 0.0,
            slash: false,
            block: false,
        }
    }

    fn delay(&self) -> f32 {
        match self.difficulty {
            AiDifficulty::Easy => 0.5,
            AiDifficulty::Normal => 0.28,
            AiDifficulty::Hard => 0.12,
        }
    }

    fn aggression(&self) -> f32 {
        match self.difficulty {
            AiDifficulty::Easy => 0.12,
            AiDifficulty::Normal => 0.30,
            AiDifficulty::Hard => 0.55,
        }
    }

    fn roll(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.rng >> 8) as f32 / (1u32 << 23) as f32
    }
}

impl System for AiControlSystem {
    fn run(&mut self, ctx: &mut TickContext<'_>) {
        let dt = ctx.dt;
        let (mut red_idx, mut blue_x) = (None, None);
        {
            let (players, transforms) = (ctx.world.store::<Player>(), ctx.world.store::<Transform>());
            let (Some(players), Some(transforms)) = (players, transforms) else {
                return;
            };
            let n = players.len().min(transforms.len());
            for i in 0..n {
                let Some(player) = players[i].as_ref() else { continue };
                let Some(body) = transforms[i].as_ref() else { continue };
                match player.side {
                    Side::Red => red_idx = Some(i),
                    Side::Blue => blue_x = Some(body.x),
                }
            }
        }
        let (Some(red_idx), Some(blue_x)) = (red_idx, blue_x) else { return };

        self.decide_at -= dt;
        if self.decide_at <= 0.0 {
            self.decide_at = self.delay();
            let me_x = ctx
                .world
                .get::<Transform>(red_idx as u32)
                .map(|t| t.x)
                .unwrap_or(blue_x);
            let gap = (blue_x - me_x).abs();
            let toward = (blue_x - me_x).signum();
            self.dir = 0.0;
            self.slash = false;
            self.block = false;
            if gap > SLASH_REACH + CUBE_SIZE {
                self.dir = toward;
            } else {
                let roll = self.roll();
                if roll < self.aggression() {
                    self.slash = true;
                } else if roll < self.aggression() + 0.35 {
                    self.block = true;
                } else {
                    self.dir = -toward;
                }
            }
        }

        let players = ctx.world.store_mut::<Player>();
        let Some(player) = players.get_mut(red_idx).and_then(Option::as_mut) else {
            return;
        };
        // wants_slash is a one-shot request — don't re-queue while a slash or
        // knockback is still executing.
        let mid_action = matches!(
            player.state,
            FighterState::Slashing | FighterState::HitStun
        );
        player.move_dir = self.dir;
        player.wants_slash = !mid_action && self.slash;
        player.wants_block =
            self.block && matches!(player.state, FighterState::Neutral | FighterState::Blocking);
    }
}