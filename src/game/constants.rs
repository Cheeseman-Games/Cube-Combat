pub const ARENA_W: f32 = 800.0;
pub const ARENA_H: f32 = 500.0;
pub const FLOOR_Y: f32 = 420.0;
pub const CUBE_SIZE: f32 = 60.0;

/// Horizontal padding between the arena walls and the spawn positions.
pub const SPAWN_PAD: f32 = 40.0;

pub const MOVE_SPEED: f32 = 280.0;
pub const BLOCK_MOVE_FACTOR: f32 = 0.4;

pub const SLASH_WINDUP: f32 = 0.08;
pub const SLASH_ACTIVE: f32 = 0.16;
pub const SLASH_RECOVER: f32 = 0.30;
pub const SLASH_TOTAL: f32 = SLASH_WINDUP + SLASH_ACTIVE + SLASH_RECOVER;
pub const SLASH_REACH: f32 = 90.0;
pub const SLASH_DAMAGE: f32 = 12.0;
pub const SLASH_CHIP: f32 = 1.0;

pub const HITSTUN: f32 = 0.28;
pub const HIT_KNOCKBACK: f32 = 240.0;
pub const BLOCK_KNOCKBACK: f32 = 50.0;
pub const KNOCK_DECAY: f32 = 0.85;

pub const MAX_HP: f32 = 100.0;