use cubic_engine::input::{FrameInput, KeyCode};
use cubic_engine::render::{Renderer, Rgba};
use crate::game::ai::AiDifficulty;
use crate::game::constants::*;
use crate::game::GameMode;

const FLASH_CHANCE: f32 = 0.03;
const FLASH_DURATION: f32 = 0.12;
const TITLE: &str = "Cube Combat";
const TITLE_LEN: usize = 11;

const BG: Rgba = Rgba::rgb(0.09, 0.10, 0.13);
const BLUE: Rgba = Rgba::rgb(0.24, 0.52, 0.95);
const RED: Rgba = Rgba::rgb(0.88, 0.30, 0.34);
const WHITE: Rgba = Rgba::rgb(0.92, 0.94, 0.96);
const MUTED: Rgba = Rgba::rgb(0.55, 0.58, 0.64);
const GOLD: Rgba = Rgba::rgb(1.0, 0.82, 0.30);

/// A random-looking seed. `SystemTime` is unsupported on wasm32 (it panics),
/// so the menu RNG is seeded with a constant here; the platform may override it
/// with e.g. the browser clock via `Menu::main_seeded`.
const DEFAULT_SEED: u32 = 0x9e37_79b9;

fn text_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.6
}

/// Player-tweakable options. Only a few knobs; tuned from the settings menu.
#[derive(Clone)]
pub struct Settings {
    pub ai: AiDifficulty,
    pub show_hints: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ai: AiDifficulty::Normal,
            show_hints: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Main,
    ModeSelect,
    Settings,
}

/// A simple menu screen finished with: the game should start.
pub enum MenuAction {
    StartGame(GameMode),
}

/// The main / mode-select / settings menu state machine. Owns the settings so
/// the player's choices carry into the next match.
pub struct Menu {
    screen: Screen,
    selected: usize,
    pub settings: Settings,
    flash: [f32; TITLE_LEN],
    rng: u32,
}

impl Menu {
    fn new(screen: Screen, seed: u32) -> Self {
        Self {
            screen,
            selected: 0,
            settings: Settings::default(),
            flash: [0.0; TITLE_LEN],
            rng: seed | 1,
        }
    }

    pub fn main() -> Self {
        Self::new(Screen::Main, DEFAULT_SEED)
    }

    /// Same as `main`, but the flash RNG is seeded by the platform (e.g. the
    /// browser clock in wasm) so the pattern differs between page loads.
    pub fn main_seeded(seed: u32) -> Self {
        Self::new(Screen::Main, seed)
    }

    fn rows(&self) -> usize {
        match self.screen {
            Screen::Main => 2,
            Screen::ModeSelect => 2,
            Screen::Settings => 2,
        }
    }

    fn roll(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.rng >> 8) as f32 / (1u32 << 23) as f32
    }

    fn advance_flash(&mut self, dt: f32) {
        for i in 0..self.flash.len() {
            if self.flash[i] > 0.0 {
                self.flash[i] -= dt;
            } else if self.roll() < FLASH_CHANCE {
                self.flash[i] = FLASH_DURATION;
            }
        }
    }

    /// Handle input and advance the menu. Returns what the application should
    /// do next, if anything.
    pub fn update(&mut self, dt: f32, frame: &FrameInput) -> Option<MenuAction> {
        self.advance_flash(dt);

        let rows = self.rows();
        if frame.pressed(KeyCode::Down) {
            self.selected = (self.selected + 1).min(rows - 1);
        }
        if frame.pressed(KeyCode::Up) {
            self.selected = self.selected.saturating_sub(1);
        }

        match self.screen {
            Screen::Main => {
                if frame.pressed(KeyCode::Enter) {
                    self.screen = if self.selected == 0 {
                        Screen::ModeSelect
                    } else {
                        Screen::Settings
                    };
                    self.selected = 0;
                }
            }
            Screen::ModeSelect => {
                if frame.pressed(KeyCode::Enter) {
                    let mode = if self.selected == 0 {
                        GameMode::TwoPlayer
                    } else {
                        GameMode::OnePlayer
                    };
                    return Some(MenuAction::StartGame(mode));
                }
                if frame.pressed(KeyCode::Escape) {
                    self.screen = Screen::Main;
                    self.selected = 0;
                }
            }
            Screen::Settings => {
                if frame.pressed(KeyCode::Left) || frame.pressed(KeyCode::Right) {
                    match self.selected {
                        0 => {
                            let dir = if frame.pressed(KeyCode::Left) { -1 } else { 1 };
                            self.settings.ai = self.settings.ai.next(dir);
                        }
                        _ => self.settings.show_hints = !self.settings.show_hints,
                    }
                }
                if frame.pressed(KeyCode::Enter) || frame.pressed(KeyCode::Escape) {
                    self.screen = Screen::Main;
                    self.selected = 0;
                }
            }
        }
        None
    }

    pub fn draw(&self, r: &mut dyn Renderer) {
        r.clear(BG);
        match self.screen {
            Screen::Main => self.draw_main(r),
            Screen::ModeSelect => self.draw_mode_select(r),
            Screen::Settings => self.draw_settings(r),
        }
    }

    fn draw_item(r: &mut dyn Renderer, label: &str, selected: bool, y: f32, size: f32) {
        let display = if selected {
            format!("> {label}")
        } else {
            label.to_owned()
        };
        let x = (ARENA_W - text_width(&display, size)) / 2.0;
        let color = if selected { GOLD } else { MUTED };
        r.text(&display, x, y, size, color);
    }

    fn draw_main(&self, r: &mut dyn Renderer) {
        let size = 64.0;
        let cw = size * 0.6;
        let x0 = (ARENA_W - text_width(TITLE, size)) / 2.0;
        let y = ARENA_H * 0.30;
        for (i, ch) in TITLE.chars().enumerate() {
            let color = if self.flash[i] > 0.0 { RED } else { BLUE };
            r.text(&ch.to_string(), x0 + i as f32 * cw, y, size, color);
        }

        let opt_y = ARENA_H * 0.55;
        Self::draw_item(r, "Play", self.selected == 0, opt_y, 30.0);
        Self::draw_item(r, "Settings", self.selected == 1, opt_y + 46.0, 30.0);

        let hint = "Up/Down: select    Enter: confirm";
        let hint_x = (ARENA_W - text_width(hint, 13.0)) / 2.0;
        r.text(hint, hint_x, ARENA_H - 30.0, 13.0, MUTED);
    }

    fn draw_mode_select(&self, r: &mut dyn Renderer) {
        let title = "SELECT MODE";
        let title_x = (ARENA_W - text_width(title, 26.0)) / 2.0;
        r.text(title, title_x, 70.0, 26.0, WHITE);

        let opt_y = ARENA_H * 0.42;
        Self::draw_item(r, "2 Players", self.selected == 0, opt_y, 30.0);
        Self::draw_item(r, "1 Player (vs AI)", self.selected == 1, opt_y + 46.0, 30.0);

        let hint = "Up/Down: select    Enter: confirm    Esc: back";
        let hint_x = (ARENA_W - text_width(hint, 13.0)) / 2.0;
        r.text(hint, hint_x, ARENA_H - 30.0, 13.0, MUTED);
    }

    fn setting_rows(&self) -> [(String, String); 2] {
        [
            ("AI Difficulty".to_owned(), self.settings.ai.label().to_owned()),
            ("Show Controls Hint".to_owned(), if self.settings.show_hints { "On" } else { "Off" }.to_owned()),
        ]
    }

    fn draw_settings(&self, r: &mut dyn Renderer) {
        let title = "SETTINGS";
        let title_x = (ARENA_W - text_width(title, 26.0)) / 2.0;
        r.text(title, title_x, 70.0, 26.0, WHITE);

        let rows = self.setting_rows();
        let y = ARENA_H * 0.42;
        for (i, (label, value)) in rows.iter().enumerate() {
            let display = format!("[ {value} ]");
            let prefix = if self.selected == i { "> " } else { "  " };
            let full = format!("{prefix}{label}: {display}");
            let x = (ARENA_W - text_width(&full, 26.0)) / 2.0;
            let color = if self.selected == i { GOLD } else { MUTED };
            r.text(&full, x, y + i as f32 * 46.0, 26.0, color);
        }

        let hint = "Left/Right: change    Enter/Esc: back";
        let hint_x = (ARENA_W - text_width(hint, 13.0)) / 2.0;
        r.text(hint, hint_x, ARENA_H - 30.0, 13.0, MUTED);
    }
}
