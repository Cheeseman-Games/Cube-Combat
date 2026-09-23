use cubic_engine::input::KeyCode;
use cubic_engine::math::Rect;
use cubic_engine::world::{EntityId, World};
use cubic_engine::{System, TickContext};
use crate::game::constants::*;
use crate::game::{AttackBox, Side};

/// Position + size of an entity. Y grows downward and the cube's feet sit on
/// the floor line, so `y = FLOOR_Y - CUBE_SIZE`.
#[derive(Clone)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Transform {
    pub fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }
}

/// High-level body state of a fighter. Drive with `move_dir` / `wants_*`
/// intents written by the control system each tick.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FighterState {
    Neutral,
    Slashing,
    Blocking,
    HitStun,
    Defeated,
}

/// All per-fighter combat / control data.
#[derive(Clone)]
pub struct Player {
    pub side: Side,
    pub hp: f32,
    pub state: FighterState,
    /// -1 faces left, +1 faces right.
    pub facing: f32,
    /// Elapsed time inside the slash timeline while `Slashing`.
    pub slash_time: f32,
    pub stun_left: f32,
    /// Instantaneous knockback velocity (px/s), decays each tick.
    pub knock_vel: f32,
    // --- intents, written by ControlSystem, consumed by the rest ---
    pub move_dir: f32,
    pub wants_slash: bool,
    pub wants_block: bool,
}

impl Player {
    pub fn new(side: Side) -> Self {
        Self {
            side,
            hp: MAX_HP,
            state: FighterState::Neutral,
            facing: if side == Side::Blue { 1.0 } else { -1.0 },
            slash_time: 0.0,
            stun_left: 0.0,
            knock_vel: 0.0,
            move_dir: 0.0,
            wants_slash: false,
            wants_block: false,
        }
    }
}

/// Maps raw key state onto fighter intents. Touches only the `Player` store.
/// In 1-player mode `red_is_ai` is set so the red cube is left to the AI
/// control system instead.
pub struct ControlSystem {
    pub red_is_ai: bool,
}

impl System for ControlSystem {
    fn run(&mut self, ctx: &mut TickContext<'_>) {
        let players = ctx.world.store_mut::<Player>();
        for player in players.iter_mut().filter_map(Option::as_mut) {
            if player.side == Side::Red && self.red_is_ai {
                continue;
            }
            let (left, right, slash, block) = match player.side {
                Side::Blue => (KeyCode::A, KeyCode::D, KeyCode::X, KeyCode::C),
                Side::Red => (
                    KeyCode::Left,
                    KeyCode::Right,
                    KeyCode::BracketLeft,
                    KeyCode::BracketRight,
                ),
            };
            player.move_dir = (ctx.input.is_down(right) as i8 - ctx.input.is_down(left) as i8) as f32;
            player.wants_slash = ctx.frame.pressed(slash);
            player.wants_block = ctx.input.is_down(block);
        }
    }
}

struct BoxSpawn {
    id: EntityId,
    side: Side,
    facing: f32,
    body: Rect,
}

/// Fighter state machine: starting slashes, blocking, hit stun timing and
/// facing. Also spawns the short-lived `AttackBox` when a slash becomes
/// active. Work buffers are owned by the system so steady-state ticks
/// allocate nothing.
#[derive(Default)]
pub struct FightSystem {
    pending: Vec<EntityId>,
    next: Vec<Player>,
    spawns: Vec<BoxSpawn>,
}

impl System for FightSystem {
    fn run(&mut self, ctx: &mut TickContext<'_>) {
        let dt = ctx.dt;
        let (players, transforms) = (ctx.world.store::<Player>(), ctx.world.store::<Transform>());
        let (Some(players), Some(transforms)) = (players, transforms) else {
            return;
        };

        self.pending.clear();
        self.next.clear();
        self.spawns.clear();

        let n = players.len().min(transforms.len());

        // Opponent-x lookup keyed by side.
        let mut x_by_side = [0.0f32; 2];
        let mut side_present = [false; 2];
        for i in 0..n {
            let Some(player) = players[i].as_ref() else { continue };
            let Some(body) = transforms[i].as_ref() else { continue };
            x_by_side[player.side.index()] = body.x;
            side_present[player.side.index()] = true;
        }

        for i in 0..n {
            let Some(player) = players[i].as_ref() else { continue };
            let Some(body) = transforms[i].as_ref() else { continue };
            let id = i as EntityId;
            let mut pl = player.clone();

            let opponent_idx = pl.side.opponent().index();
            let opponent_x = if side_present[opponent_idx] {
                x_by_side[opponent_idx]
            } else {
                body.x
            };
            pl.facing = if opponent_x > body.x { 1.0 } else { -1.0 };

            match pl.state {
                FighterState::Neutral => {
                    if pl.wants_slash {
                        pl.state = FighterState::Slashing;
                        pl.slash_time = 0.0;
                        pl.move_dir = 0.0;
                    } else if pl.wants_block {
                        pl.state = FighterState::Blocking;
                    }
                }
                FighterState::Blocking => {
                    if !pl.wants_block {
                        pl.state = FighterState::Neutral;
                    }
                }
                FighterState::Slashing => {
                    let before = pl.slash_time;
                    pl.slash_time += dt;
                    if before < SLASH_WINDUP && pl.slash_time >= SLASH_WINDUP {
                        self.spawns.push(BoxSpawn {
                            id,
                            side: pl.side,
                            facing: pl.facing,
                            body: body.rect(),
                        });
                    }
                    if pl.slash_time >= SLASH_TOTAL {
                        pl.state = FighterState::Neutral;
                    }
                    pl.move_dir = 0.0;
                }
                FighterState::HitStun => {
                    pl.stun_left -= dt;
                    if pl.stun_left <= 0.0 {
                        pl.state = FighterState::Neutral;
                        pl.knock_vel = 0.0;
                    }
                }
                FighterState::Defeated => {
                    pl.move_dir = 0.0;
                }
            }

            self.pending.push(id);
            self.next.push(pl);
        }

        for (&id, pl) in self.pending.iter().zip(self.next.drain(..)) {
            ctx.world.insert::<Player>(id, pl);
        }
        for spawn in self.spawns.drain(..) {
            Self::spawn_attack_box(ctx.world, &spawn);
        }
    }
}

impl FightSystem {
    fn spawn_attack_box(world: &mut World, spawn: &BoxSpawn) {
        let front = if spawn.facing > 0.0 {
            spawn.body.x + spawn.body.w
        } else {
            spawn.body.x
        };
        let box_x = if spawn.facing > 0.0 { front } else { front - SLASH_REACH };
        world.insert(
            spawn.id,
            AttackBox {
                owner: spawn.id,
                side: spawn.side,
                rect: Rect::new(box_x, spawn.body.y, SLASH_REACH, spawn.body.h),
                damage: SLASH_DAMAGE,
                remaining: SLASH_ACTIVE,
                hit: false,
            },
        );
    }
}