use crate::engine::input::{FrameInput, InputState, KeyCode};
use crate::engine::render::Renderer;
use crate::engine::GameDriver;
use crate::game::ai::AiDifficulty;
use crate::game::Game;
use crate::game::GameMode;
use crate::menus::{Menu, MenuAction};

enum Screen {
    Menu(Menu),
    Playing(Game),
}

/// Top-level game shell: hosts the menu screens and the match, switching
/// between them based on user input.
pub struct App {
    screen: Screen,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Menu(Menu::main()),
        }
    }

    /// Same as `new`, with a platform-provided seed so menu effects (title
    /// flashing) differ between runs instead of replaying the same pattern.
    pub fn new_seeded(seed: u32) -> Self {
        Self {
            screen: Screen::Menu(Menu::main_seeded(seed)),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl GameDriver for App {
    fn simulate(&mut self, dt: f32, input: &InputState, frame: &FrameInput) {
        let mut launch: Option<(GameMode, AiDifficulty, bool)> = None;
        let mut back_to_menu = false;

        match &mut self.screen {
            Screen::Menu(menu) => {
                if let Some(MenuAction::StartGame(mode)) = menu.update(dt, frame) {
                    launch = Some((mode, menu.settings.ai, menu.settings.show_hints));
                }
            }
            Screen::Playing(game) => {
                if frame.pressed(KeyCode::Escape) {
                    back_to_menu = true;
                } else {
                    game.simulate(dt, input, frame);
                }
            }
        }

        if let Some((mode, ai, hints)) = launch {
            self.screen = Screen::Playing(Game::with(mode, ai, hints));
        } else if back_to_menu {
            self.screen = Screen::Menu(Menu::main());
        }
    }

    fn draw(&self, renderer: &mut dyn Renderer) {
        match &self.screen {
            Screen::Menu(menu) => menu.draw(renderer),
            Screen::Playing(game) => game.draw(renderer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::input::InputState;
    use crate::engine::render::DrawList;

    fn tap(input: &mut InputState, keys: &[KeyCode]) {
        for &key in keys {
            input.key_down(key);
            input.key_up(key);
        }
    }

    /// Boots the app, paints each menu, navigates every screen, starts a match
    /// and quits it — a smoke test for the whole menu layer.
    #[test]
    fn app_boots_navigates_and_paints() {
        let mut app = App::new();
        let mut input = InputState::new();
        let mut list = DrawList::new();
        let dt = 1.0 / 60.0;

        let tick = |app: &mut App, input: &mut InputState, list: &mut DrawList| {
            let frame = input.begin_frame();
            app.simulate(dt, input, &frame);
            app.draw(list);
        };

        // Main menu paints.
        tick(&mut app, &mut input, &mut list);

        // Main -> mode select.
        tap(&mut input, &[KeyCode::Enter]);
        tick(&mut app, &mut input, &mut list);

        // Mode select -> start a 1-player match.
        tap(&mut input, &[KeyCode::Down, KeyCode::Enter]);
        tick(&mut app, &mut input, &mut list);
        assert!(matches!(app.screen, Screen::Playing(_)));

        // Paint the match, then quit back to the main menu.
        tick(&mut app, &mut input, &mut list);
        tap(&mut input, &[KeyCode::Escape]);
        tick(&mut app, &mut input, &mut list);
        assert!(matches!(app.screen, Screen::Menu(_)));

        // Settings: cycle difficulty + toggle hints, then back.
        tap(&mut input, &[KeyCode::Down, KeyCode::Enter]);
        tick(&mut app, &mut input, &mut list);
        tap(&mut input, &[KeyCode::Right, KeyCode::Right]);
        tick(&mut app, &mut input, &mut list);
        tap(&mut input, &[KeyCode::Down, KeyCode::Right]);
        tick(&mut app, &mut input, &mut list);
        tap(&mut input, &[KeyCode::Escape]);
        tick(&mut app, &mut input, &mut list);
        assert!(matches!(app.screen, Screen::Menu(_)));
    }
}