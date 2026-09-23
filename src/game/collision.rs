use cubic_engine::world::EntityId;
use cubic_engine::{System, TickContext};
use crate::game::constants::*;
use crate::game::fighters::{FighterState, Player, Transform};
use crate::game::{AttackBox};

struct Blow {
    id: EntityId,
    hp: f32,
    stun_left: f32,
    knock_vel: f32,
    blocked: bool,
}

/// Ages and resolves slash hit-boxes against the opposing fighter. Collected
/// hit results are applied with one write pass per store; consumed boxes are
/// pruned. All scratch buffers are owned and reused.
#[derive(Default)]
pub struct AttackCollisionSystem {
    alive_boxes: Vec<(EntityId, AttackBox)>,
    blows: Vec<Blow>,
    consumed: Vec<EntityId>,
}

impl System for AttackCollisionSystem {
    fn run(&mut self, ctx: &mut TickContext<'_>) {
        let dt = ctx.dt;
        self.alive_boxes.clear();
        self.blows.clear();
        self.consumed.clear();

        // Age boxes; drop expired or already-hit ones.
        {
            let boxes = ctx.world.store_mut::<AttackBox>();
            for (id, slot) in boxes.iter_mut().enumerate() {
                if let Some(boxx) = slot.as_mut() {
                    boxx.remaining -= dt;
                    if boxx.remaining <= 0.0 || boxx.hit {
                        *slot = None;
                    } else {
                        self.alive_boxes.push((id as EntityId, boxx.clone()));
                    }
                }
            }
        }

        // Resolve intersections.
        {
            let (players, transforms) =
                (ctx.world.store::<Player>(), ctx.world.store::<Transform>());
            let (Some(players), Some(transforms)) = (players, transforms) else {
                return;
            };

            for &(box_id, ref boxx) in &self.alive_boxes {
                let n = players.len().min(transforms.len());
                for i in 0..n {
                    if i == box_id as usize {
                        continue;
                    }
                    let Some(player) = players[i].as_ref() else { continue };
                    let Some(body) = transforms[i].as_ref() else { continue };
                    if player.state == FighterState::Defeated || player.side == boxx.side {
                        continue;
                    }
                    if !boxx.rect.intersects(&body.rect()) {
                        continue;
                    }

                    let away = if boxx.rect.center_x() > body.rect().center_x() {
                        -1.0
                    } else {
                        1.0
                    };
                    match player.state {
                        FighterState::Blocking => {
                            self.blows.push(Blow {
                                id: i as EntityId,
                                hp: player.hp - SLASH_CHIP,
                                stun_left: HITSTUN,
                                knock_vel: away * BLOCK_KNOCKBACK,
                                blocked: true,
                            });
                        }
                        _ => {
                            let hp = player.hp - boxx.damage;
                            self.blows.push(Blow {
                                id: i as EntityId,
                                hp,
                                stun_left: HITSTUN,
                                knock_vel: away * HIT_KNOCKBACK,
                                blocked: false,
                            });
                        }
                    }
                    self.consumed.push(box_id);
                    break;
                }
            }
        }

        // Apply blows.
        if !self.blows.is_empty() {
            let players = ctx.world.store_mut::<Player>();
            for blow in &self.blows {
                if let Some(player) = players
                    .get_mut(blow.id as usize)
                    .and_then(Option::as_mut)
                {
                    if blow.blocked {
                        player.hp = blow.hp;
                        player.stun_left = blow.stun_left;
                        player.knock_vel = blow.knock_vel;
                        continue;
                    }
                    let defeated = blow.hp <= 0.0;
                    player.hp = blow.hp;
                    player.state = if defeated {
                        FighterState::Defeated
                    } else {
                        FighterState::HitStun
                    };
                    player.stun_left = blow.stun_left;
                    player.knock_vel = blow.knock_vel;
                }
            }
        }

        // Prune consumed boxes (marking hit already removed them next tick;
        // drop now so the frame state is exact).
        if !self.consumed.is_empty() {
            let boxes = ctx.world.store_mut::<AttackBox>();
            for &id in &self.consumed {
                if let Some(slot) = boxes.get_mut(id as usize) {
                    *slot = None;
                }
            }
        }
    }
}