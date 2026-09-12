use crate::engine::render::{Renderer, Rgba};
use crate::game::constants::*;
use crate::game::fighters::{FighterState, Player, Transform};
use crate::game::{AttackBox, Game, MatchState, Side};

const BG: Rgba = Rgba::rgb(0.09, 0.10, 0.13);
const FLOOR: Rgba = Rgba::new(1.0, 1.0, 1.0, 0.15);
const BAR_BG: Rgba = Rgba::new(1.0, 1.0, 1.0, 0.12);
const WHITE: Rgba = Rgba::rgb(0.92, 0.94, 0.96);
const MUTED: Rgba = Rgba::rgb(0.55, 0.58, 0.64);
const GOLD: Rgba = Rgba::rgb(1.0, 0.82, 0.30);

struct Palette {
    body: Rgba,
    edge: Rgba,
    accent: Rgba,
}

fn palette(side: Side) -> Palette {
    match side {
        Side::Blue => Palette {
            body: Rgba::rgb(0.24, 0.52, 0.95),
            edge: Rgba::rgb(0.62, 0.84, 1.0),
            accent: Rgba::new(0.62, 0.84, 1.0, 0.55),
        },
        Side::Red => Palette {
            body: Rgba::rgb(0.88, 0.30, 0.34),
            edge: Rgba::rgb(1.0, 0.66, 0.62),
            accent: Rgba::new(1.0, 0.66, 0.62, 0.55),
        },
    }
}

fn hp_bar(r: &mut dyn Renderer, x: f32, y: f32, w: f32, fraction: f32, color: Rgba) {
    r.fill_rect(x, y, w, 14.0, BAR_BG);
    let fill = (w * fraction.clamp(0.0, 1.0)).max(if fraction > 0.0 { 2.0 } else { 0.0 });
    r.fill_rect(x, y, fill, 14.0, color);
}

pub fn draw(game: &Game, r: &mut dyn Renderer) {
    r.clear(BG);

    let bar_w = 260.0;
    let bar_y = 18.0;
    r.text("CUBE COMBAT", ARENA_W / 2.0 - 60.0, 2.0, 16.0, MUTED);

    for side in [Side::Blue, Side::Red] {
        let id = game.entity_for(side);
        let Some(player) = game.world().get::<Player>(id) else {
            continue;
        };
        let Some(body) = game.world().get::<Transform>(id) else {
            continue;
        };
        let pal = palette(side);
        let is_left = side == Side::Blue;

        let label_x = if is_left { 40.0 } else { ARENA_W - 40.0 - bar_w };
        let name = if is_left { "BLUE" } else { "RED" };
        r.text(
            name,
            label_x,
            bar_y - 16.0,
            14.0,
            if is_left { pal.edge } else { pal.body },
        );

        hp_bar(r, label_x, bar_y, bar_w, player.hp / MAX_HP, pal.body);
        r.fill_rect(if is_left { label_x - 6.0 } else { label_x + bar_w + 6.0 }, bar_y, 4.0, 14.0, WHITE);
        r.text(
            &format!("wins: {}", game.wins(side)),
            label_x,
            bar_y + 20.0,
            12.0,
            MUTED,
        );

        match player.state {
            FighterState::Blocking => {
                r.fill_rect(body.x - 4.0, body.y - 4.0, body.w + 8.0, body.h + 8.0, Rgba::new(1.0, 0.82, 0.30, 0.28));
            }
            FighterState::Slashing => {
                r.fill_rect(body.x - 3.0, body.y - 3.0, body.w + 6.0, body.h + 6.0, Rgba::new(1.0, 1.0, 1.0, 0.25));
            }
            _ => {}
        }

        if player.state == FighterState::HitStun {
            r.fill_rect(body.x, body.y, body.w, body.h, Rgba::new(1.0, 1.0, 1.0, 0.35));
        }

        r.fill_rect(body.x, body.y, body.w, body.h, pal.body);
        let edge_w = 3.0;
        r.fill_rect(body.x, body.y, body.w, edge_w, pal.edge);
        r.fill_rect(body.x, body.y + body.h - edge_w, body.w, edge_w, pal.edge);
        r.fill_rect(body.x, body.y, edge_w, body.h, pal.edge);
        r.fill_rect(body.x + body.w - edge_w, body.y, edge_w, body.h, pal.edge);
    }

    for (_, slash) in game.world().iter::<AttackBox>() {
        let pal = palette(slash.side);
        r.fill_rect(slash.rect.x, slash.rect.y, slash.rect.w, slash.rect.h, pal.accent);
        r.fill_rect(
            slash.rect.x + 10.0,
            slash.rect.y + 10.0,
            slash.rect.w - 20.0,
            slash.rect.h,
            Rgba::new(1.0, 1.0, 1.0, 0.30),
        );
    }

    r.fill_rect(0.0, FLOOR_Y, ARENA_W, 3.0, FLOOR);

    r.text(
        "P1: A/D move   X slash   C block    |    P2: arrows move   [ slash   ] block    |    Enter: rematch",
        22.0,
        ARENA_H - 26.0,
        13.0,
        MUTED,
    );

    if let MatchState::Ended(winner) = game.state() {
        let col = palette(winner);
        let label = match winner {
            Side::Blue => "BLUE WINS",
            Side::Red => "RED WINS",
        };
        r.text(label, ARENA_W / 2.0 - 110.0, ARENA_H / 2.0 - 60.0, 44.0, col.body);
        r.text("press Enter to rematch", ARENA_W / 2.0 - 120.0, ARENA_H / 2.0 + 18.0, 18.0, GOLD);
    }
}