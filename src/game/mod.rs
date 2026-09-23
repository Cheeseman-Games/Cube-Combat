pub mod ai;
pub mod collision;
pub mod constants;
pub mod draw;
pub mod fighters;
pub mod movement;

use cubic_engine::input::{FrameInput, InputState, KeyCode};
use cubic_engine::math::Rect;
use cubic_engine::world::{EntityId, World};
use cubic_engine::{System, TickContext};
use crate::game::ai::AiDifficulty;
use crate::game::constants::*;
use crate::game::fighters::{Player, Transform};

/// How a match is played. In 1-player mode the red cube is driven by AI.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameMode {
    TwoPlayer,
    OnePlayer,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Side {
    Blue,
    Red,
}

impl Side {
    pub fn opponent(self) -> Side {
        match self {
            Side::Blue => Side::Red,
            Side::Red => Side::Blue,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Side::Blue => 0,
            Side::Red => 1,
        }
    }
}

/// Short-lived attack hitbox spawned while a slash is active.
#[derive(Clone)]
pub struct AttackBox {
    pub owner: EntityId,
    pub side: Side,
    pub rect: Rect,
    pub damage: f32,
    pub remaining: f32,
    pub hit: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MatchState {
    Playing,
    Ended(Side),
}

/// The game itself: an entity world plus an ordered system pipeline.
///
/// New systems go into `Game::new` and run every tick in the order listed.
pub struct Game {
    world: World,
    systems: Vec<Box<dyn System>>,
    blue: EntityId,
    red: EntityId,
    wins: [u32; 2],
    state: MatchState,
    show_hints: bool,
}

impl Game {
    pub fn new() -> Self {
        Self::with(GameMode::TwoPlayer, AiDifficulty::Normal, true)
    }

    pub fn with(mode: GameMode, ai: AiDifficulty, show_hints: bool) -> Self {
        let mut world = World::new();

        let blue = Self::spawn_fighter(&mut world, Side::Blue, SPAWN_PAD);
        let red = Self::spawn_fighter(&mut world, Side::Red, ARENA_W - SPAWN_PAD - CUBE_SIZE);

        let mut systems: Vec<Box<dyn System>> = vec![Box::new(fighters::ControlSystem {
            red_is_ai: mode == GameMode::OnePlayer,
        })];
        if mode == GameMode::OnePlayer {
            systems.push(Box::new(ai::AiControlSystem::new(ai)));
        }
        systems.push(Box::new(fighters::FightSystem::default()));
        systems.push(Box::new(movement::MovementSystem::default()));
        systems.push(Box::new(collision::AttackCollisionSystem::default()));

        Self {
            world,
            systems,
            blue,
            red,
            wins: [0, 0],
            state: MatchState::Playing,
            show_hints,
        }
    }

    fn spawn_fighter(world: &mut World, side: Side, x: f32) -> EntityId {
        let id = world.spawn();
        world.insert(
            id,
            Transform {
                x,
                y: FLOOR_Y - CUBE_SIZE,
                w: CUBE_SIZE,
                h: CUBE_SIZE,
            },
        );
        world.insert(id, Player::new(side));
        id
    }

    pub fn entity_for(&self, side: Side) -> EntityId {
        match side {
            Side::Blue => self.blue,
            Side::Red => self.red,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn state(&self) -> MatchState {
        self.state
    }

    pub fn wins(&self, side: Side) -> u32 {
        self.wins[match side {
            Side::Blue => 0,
            Side::Red => 1,
        }]
    }

    pub fn show_hints(&self) -> bool {
        self.show_hints
    }

    /// Advance one fixed simulation step.
    pub fn simulate(&mut self, dt: f32, input: &InputState, frame: &FrameInput) {
        if let MatchState::Ended(_) = self.state {
            if frame.pressed(KeyCode::Enter) {
                self.reset_match();
            }
            return;
        }

        let mut systems = std::mem::take(&mut self.systems);
        for system in systems.iter_mut() {
            let mut ctx = TickContext {
                world: &mut self.world,
                input,
                frame,
                dt,
            };
            system.run(&mut ctx);
        }
        self.systems = systems;

        self.resolve_defeat();
    }

    /// Render the game to the given backend.
    pub fn draw(&self, renderer: &mut dyn cubic_engine::render::Renderer) {
        draw::draw(self, renderer);
    }

    fn resolve_defeat(&mut self) {
        let blue_hp = self.world.get::<Player>(self.blue).map(|p| p.hp).unwrap_or(0.0);
        let red_hp = self.world.get::<Player>(self.red).map(|p| p.hp).unwrap_or(0.0);

        let winner = if blue_hp <= 0.0 {
            Some(Side::Red)
        } else if red_hp <= 0.0 {
            Some(Side::Blue)
        } else {
            None
        };

        if let Some(winner) = winner {
            self.wins[match winner {
                Side::Blue => 0,
                Side::Red => 1,
            }] += 1;
            self.state = MatchState::Ended(winner);
        }
    }

    fn reset_match(&mut self) {
        for side in [Side::Blue, Side::Red] {
            let id = self.entity_for(side);
            if let Some(player) = self.world.get_mut::<Player>(id) {
                *player = Player::new(side);
            }
            self.world.remove::<AttackBox>(id);
            let x = match side {
                Side::Blue => SPAWN_PAD,
                Side::Red => ARENA_W - SPAWN_PAD - CUBE_SIZE,
            };
            if let Some(t) = self.world.get_mut::<Transform>(id) {
                t.x = x;
                t.y = FLOOR_Y - CUBE_SIZE;
            }
        }
        self.state = MatchState::Playing;
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::fighters::FighterState;

    fn tick(game: &mut Game, input: &mut InputState, count: u32) {
        let dt = 1.0 / 60.0;
        for _ in 0..count {
            let frame = input.begin_frame();
            game.simulate(dt, input, &frame);
        }
    }

    fn press(input: &mut InputState, keys: &[KeyCode]) {
        for &k in keys {
            input.key_down(k);
        }
    }

    fn hp(game: &Game, side: Side) -> f32 {
        game.world().get::<Player>(game.entity_for(side)).unwrap().hp
    }

    fn x_of(game: &Game, side: Side) -> f32 {
        game.world().get::<Transform>(game.entity_for(side)).unwrap().x
    }

    fn set_pos(game: &mut Game, side: Side, x: f32) {
        let id = game.entity_for(side);
        game.world_mut().get_mut::<Transform>(id).unwrap().x = x;
    }

    fn set_hp(game: &mut Game, side: Side, hp: f32) {
        let id = game.entity_for(side);
        game.world_mut().get_mut::<Player>(id).unwrap().hp = hp;
    }

    #[test]
    fn slash_from_blue_damages_red() {
        let mut game = Game::new();
        let mut input = InputState::new();
        set_pos(&mut game, Side::Red, 150.0);

        press(&mut input, &[KeyCode::X]);
        // Advance through windup + active + recover.
        tick(&mut game, &mut input, 30);

        assert!(hp(&game, Side::Red) < MAX_HP, "red should have taken damage");
    }

    #[test]
    fn blocking_negates_slash_damage() {
        let mut game = Game::new();
        let mut input = InputState::new();
        set_pos(&mut game, Side::Red, 150.0);
        // Red holds block (right bracket) before / while being slashed.
        press(&mut input, &[KeyCode::X, KeyCode::BracketRight]);

        tick(&mut game, &mut input, 30);

        let hp_after = hp(&game, Side::Red);
        // Only chip damage, no full hit and no hit-stun.
        assert_eq!(hp_after, MAX_HP - SLASH_CHIP, "blocked hit should only chip");
        assert_eq!(
            game.world().get::<Player>(game.entity_for(Side::Red)).unwrap().state,
            FighterState::Blocking
        );
    }

    #[test]
    fn fighters_stay_inside_arena() {
        let mut game = Game::new();
        let mut input = InputState::new();

        press(&mut input, &[KeyCode::A]);
        tick(&mut game, &mut input, 600);
        assert!(x_of(&game, Side::Blue) >= 0.0);

        // Move blue as far right as possible.
        let mut input2 = InputState::new();
        press(&mut input2, &[KeyCode::D]);
        tick(&mut game, &mut input2, 600);
        assert!(x_of(&game, Side::Blue) <= ARENA_W - CUBE_SIZE);
    }

    #[test]
    fn match_ends_when_hp_reaches_zero() {
        let mut game = Game::new();
        let mut input = InputState::new();
        set_hp(&mut game, Side::Red, 10.0);
        set_pos(&mut game, Side::Red, 150.0);

        press(&mut input, &[KeyCode::X]);
        tick(&mut game, &mut input, 30);

        assert_eq!(game.state(), MatchState::Ended(Side::Blue));
        assert_eq!(game.wins(Side::Blue), 1);
    }

    #[test]
    fn enter_rematches_after_victory() {
        let mut game = Game::new();
        let mut input = InputState::new();
        set_hp(&mut game, Side::Red, 1.0);
        set_pos(&mut game, Side::Red, 150.0);

        press(&mut input, &[KeyCode::X]);
        tick(&mut game, &mut input, 30);
        assert_eq!(game.state(), MatchState::Ended(Side::Blue));

        press(&mut input, &[KeyCode::Enter]);
        tick(&mut game, &mut input, 1);

        assert_eq!(game.state(), MatchState::Playing);
        assert_eq!(hp(&game, Side::Red), MAX_HP);
        assert_eq!(game.wins(Side::Blue), 1, "wins persist across rematches");
    }

    #[test]
    fn slashing_locks_movement() {
        let mut game = Game::new();
        let mut input = InputState::new();
        let start = x_of(&game, Side::Blue);

        press(&mut input, &[KeyCode::D, KeyCode::X]);
        tick(&mut game, &mut input, 10);

        assert_eq!(x_of(&game, Side::Blue), start, "cannot move while slashing");
    }

    #[test]
    fn ai_drives_red_toward_blue() {
        let mut game = Game::with(GameMode::OnePlayer, AiDifficulty::Normal, true);
        let mut input = InputState::new();
        let start = x_of(&game, Side::Red);

        press(&mut input, &[KeyCode::D]);
        tick(&mut game, &mut input, 60);

        assert!(
            x_of(&game, Side::Red) < start,
            "AI should move red toward the idle blue cube"
        );
    }
}
