//! SPEC 6 第 2 ～ 3 步：UI 动作解析与放置 / 铲除。

use std::collections::BTreeSet;

use trellia_core::{Entity, World};

use crate::battle::defender_bundle;
use crate::components::{Cell, DefenderRef, Health, OverlayOf, Shielded};
use crate::data::{Behavior, Placement};
use crate::events::{DefenderPlaced, DefenderRemoved, RemoveCause, UiAction};
use crate::grid::Grid;
use crate::registry::DefenderRegistry;
use crate::resources::{BattleState, CardTray, LaneState, PendingOps};

/// SPEC 6 第 2 步：取走全部 `UiAction`，解析对局动作；场景动作原样放回供场景消费（ADR-002）。
pub fn ui_action_system(world: &mut World) {
    let actions: Vec<UiAction> = world.events.get_mut::<UiAction>().drain().collect();
    let mut passthrough = Vec::new();
    for action in actions {
        if !action.is_battle_action() {
            passthrough.push(action);
            continue;
        }
        let ready: Vec<bool> = world
            .resources
            .expect::<CardTray>()
            .cards
            .iter()
            .map(|c| c.ready)
            .collect();
        let state = world.resources.expect_mut::<BattleState>();
        match action {
            UiAction::SelectCard(i) => {
                if ready.get(i).copied().unwrap_or(false) {
                    state.selected_card = Some(i);
                    state.shovel_mode = false;
                }
            }
            UiAction::DeselectCard => {
                state.selected_card = None;
                state.shovel_mode = false;
            }
            UiAction::ToggleShovel => {
                state.shovel_mode = !state.shovel_mode;
                state.selected_card = None;
            }
            UiAction::PlaceAt { row, col } => {
                if let Some(i) = state.selected_card {
                    world
                        .resources
                        .expect_mut::<PendingOps>()
                        .place
                        .push((i, Cell { row, col }));
                }
            }
            UiAction::ShovelAt { row, col } => {
                world
                    .resources
                    .expect_mut::<PendingOps>()
                    .shovel
                    .push(Cell { row, col });
            }
            UiAction::CollectDewAt(pos) => {
                world.resources.expect_mut::<PendingOps>().collect.push(pos);
            }
            _ => {}
        }
    }
    let events = world.events.get_mut::<UiAction>();
    for action in passthrough {
        events.send(action);
    }
}

/// 某格现有的防御者：(地面防御者, 覆盖层)，只计存活者。
fn occupants(world: &World, cell: Cell) -> (Option<Entity>, Option<Entity>) {
    let mut ground = None;
    let mut overlay = None;
    for (e, c, h, over) in world
        .query::<(Entity, &Cell, &Health, Option<&OverlayOf>)>()
        .iter()
    {
        if *c != cell || !h.alive() {
            continue;
        }
        if over.is_some() {
            overlay = Some(e);
        } else {
            ground = Some(e);
        }
    }
    (ground, overlay)
}

/// 移除一个防御者：生命置 0（本 tick 后续系统视为不存在），tick 末销毁，发 `DefenderRemoved`。
/// 宿主被移除时其覆盖层一并移除；覆盖层被移除时解除宿主的 `Shielded`。
pub(crate) fn remove_defender(world: &mut World, e: Entity, cause: RemoveCause) {
    let Some((id, cell)) = world
        .entities
        .query_one_mut::<(&DefenderRef, &Cell)>(e)
        .ok()
        .map(|(d, c)| (d.id, *c))
    else {
        return;
    };
    if let Some(mut h) = world.get_mut::<Health>(e) {
        h.hp = 0;
    }
    world.commands.despawn(e);
    world.events.send(DefenderRemoved {
        entity: e,
        id,
        cell,
        cause,
    });
    let host = world.get::<OverlayOf>(e).map(|o| o.host);
    if let Some(host) = host {
        world.commands.remove::<(Shielded,)>(host);
    }
    let overlay = world.get::<Shielded>(e).map(|s| s.overlay);
    if let Some(overlay) = overlay
        && world.get::<Health>(overlay).is_some_and(|h| h.alive())
    {
        remove_defender(world, overlay, cause);
    }
}

/// SPEC 6 第 3 步：校验并执行铲除与放置（SPEC 7.2 Overlay、9 铲子）。
///
/// 新防御者通过 `reserve_entity` + 延迟插入在本 tick 末生效，下一 tick 起参与模拟。
pub fn placement_system(world: &mut World) {
    let ops = std::mem::take(world.resources.expect_mut::<PendingOps>());
    world.resources.expect_mut::<PendingOps>().collect = ops.collect;

    for cell in ops.shovel {
        let (ground, overlay) = occupants(world, cell);
        if let Some(target) = overlay.or(ground) {
            remove_defender(world, target, RemoveCause::Shovel);
            world.resources.expect_mut::<BattleState>().shovel_mode = false;
        }
    }

    let mut placed_this_tick: BTreeSet<Cell> = BTreeSet::new();
    for (slot, cell) in ops.place {
        try_place(world, slot, cell, &mut placed_this_tick);
    }
}

fn try_place(world: &mut World, slot: usize, cell: Cell, placed: &mut BTreeSet<Cell>) {
    if cell.row >= Grid::ROWS
        || cell.col >= Grid::COLS
        || !world.resources.expect::<LaneState>().is_active(cell.row)
    {
        return;
    }
    let Some(card) = world
        .resources
        .expect::<CardTray>()
        .cards
        .get(slot)
        .copied()
    else {
        return;
    };
    let Some(def) = world
        .resources
        .expect::<DefenderRegistry>()
        .get(card.defender)
        .cloned()
    else {
        return;
    };
    let dew = world.resources.expect::<BattleState>().dew;
    if !card.cooldown.finished() || dew < def.cost {
        return;
    }
    let (ground, overlay) = occupants(world, cell);

    let host = match def.placement {
        Placement::Ground | Placement::Any => {
            if ground.is_some() || placed.contains(&cell) {
                return;
            }
            None
        }
        Placement::Overlay => {
            let Some(host) = ground else {
                return;
            };
            if let Some(existing) = overlay {
                // 修复：仅当现有覆盖层 hp < num/den × max（SPEC 7.1 Gourdshell）
                let (num, den) = def
                    .behaviors
                    .iter()
                    .find_map(|b| match *b {
                        Behavior::Overlay {
                            repair_below_num,
                            repair_below_den,
                        } => Some((repair_below_num, repair_below_den)),
                        _ => None,
                    })
                    .unwrap_or((1, 1));
                let Some(mut h) = world.get_mut::<Health>(existing) else {
                    return;
                };
                if h.hp * den >= h.max * num {
                    return;
                }
                h.hp = h.max;
                drop(h);
                pay(world, slot, def.cost);
                world.events.send(DefenderPlaced {
                    entity: existing,
                    id: def.id,
                    cell,
                });
                return;
            }
            if placed.contains(&cell) {
                return;
            }
            Some(host)
        }
    };

    let entity = world.entities.reserve_entity();
    let mut builder = defender_bundle(&def, cell, host);
    world.commands.insert(entity, builder.build());
    if let Some(host) = host {
        world.commands.insert(host, (Shielded { overlay: entity },));
    }
    placed.insert(cell);
    pay(world, slot, def.cost);
    world.events.send(DefenderPlaced {
        entity,
        id: def.id,
        cell,
    });
}

/// 扣 Dew、重置卡牌冷却、取消选卡（SPEC 9）。
fn pay(world: &mut World, slot: usize, cost: i32) {
    let state = world.resources.expect_mut::<BattleState>();
    state.dew -= cost;
    state.selected_card = None;
    if let Some(card) = world.resources.expect_mut::<CardTray>().cards.get_mut(slot) {
        card.cooldown.reset();
        card.ready = false;
    }
}
