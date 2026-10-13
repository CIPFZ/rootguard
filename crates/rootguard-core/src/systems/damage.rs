//! SPEC 6 第 11 ～ 13 步：伤害统一结算、死亡处理、车道防御（Hose Burst）。

use trellia_core::{Entity, Transform2D, World};

use crate::battle::spawn_enemy;
use crate::components::{Armor, Blast, Carrier, DefenderRef, EnemyRef, Health, Heavy, Lane};
use crate::data::ArmorKind;
use crate::events::{
    BattleEnded, DamageSource, EnemyArmorBroken, EnemyDamaged, EnemyDied, HoseTriggered, Outcome,
    RemoveCause,
};
use crate::grid::Grid;
use crate::resources::{
    BattleState, DamageQueue, DamageRequest, Death, DeathLog, LaneState, Phase,
};
use crate::systems::input::remove_defender;

/// SPEC 6 第 11 步：按 `(target, seq)` 统一结算本 tick 全部伤害请求。
///
/// 敌人：护甲 → 主生命，溢出部分落到主生命（SPEC 2.4 / 8.3：Mine / Blast 无视护甲，Spikes 无视
/// Shield，Chomp / Hose 即死，Heavy 免疫 Chomp），发 `EnemyDamaged / EnemyArmorBroken / EnemyDied`。
/// 防御者：Blast 引信期间不掉血（SPEC 7.2）；归零发 `DefenderRemoved(Killed)`。
pub fn damage_system(world: &mut World) {
    let requests = world.resources.expect_mut::<DamageQueue>().take_sorted();
    for req in requests {
        if world.get::<EnemyRef>(req.target).is_some() {
            damage_enemy(world, req);
        } else if world.get::<DefenderRef>(req.target).is_some() {
            damage_defender(world, req);
        }
    }
}

fn damage_enemy(world: &mut World, req: DamageRequest) {
    let target = req.target;
    let Some(hp) = world.get::<Health>(target).map(|h| h.hp) else {
        return;
    };
    if hp <= 0 {
        return;
    }
    let heavy = world.get::<Heavy>(target).is_some();
    let mut armor = world.get::<Armor>(target).map(|a| *a).filter(|a| a.hp > 0);
    let mut absorbed = 0;
    let to_hp = match req.source {
        DamageSource::Chomp if heavy => return,
        DamageSource::Chomp | DamageSource::Hose => hp,
        _ => {
            let amount = req.amount.max(0);
            let bypass = req.ignore_armor
                || (req.source == DamageSource::Spikes
                    && armor.is_some_and(|a| a.kind == ArmorKind::Shield));
            if !bypass && let Some(a) = armor.as_mut() {
                absorbed = amount.min(a.hp);
                a.hp -= absorbed;
            }
            amount - absorbed
        }
    };
    let broken = armor.filter(|a| absorbed > 0 && a.hp == 0).map(|a| a.kind);
    if let Some(a) = armor
        && let Some(mut comp) = world.get_mut::<Armor>(target)
    {
        comp.hp = a.hp;
    }
    let new_hp = hp - to_hp;
    if let Some(mut h) = world.get_mut::<Health>(target) {
        h.hp = new_hp;
    }
    world.events.send(EnemyDamaged {
        entity: target,
        amount: absorbed + to_hp,
        source: req.source,
    });
    if let Some(kind) = broken {
        world.commands.remove::<(Armor,)>(target);
        world.events.send(EnemyArmorBroken {
            entity: target,
            kind,
        });
    }
    if new_hp <= 0 {
        record_death(world, target, req.source != DamageSource::Hose);
    }
}

/// 发 `EnemyDied` 并记入 `DeathLog`（由 `death_system` 销毁与衍生）。
fn record_death(world: &mut World, e: Entity, spawn_children: bool) {
    let Some((id, lane, x)) = world
        .entities
        .query_one_mut::<(&EnemyRef, &Lane, &Transform2D)>(e)
        .ok()
        .map(|(r, l, t)| (r.id, l.row, t.pos.x))
    else {
        return;
    };
    world.events.send(EnemyDied {
        entity: e,
        id,
        lane,
        x,
    });
    world.resources.expect_mut::<DeathLog>().0.push(Death {
        entity: e,
        id,
        lane,
        x,
        spawn_children,
    });
}

fn damage_defender(world: &mut World, req: DamageRequest) {
    let target = req.target;
    if world.get::<Blast>(target).is_some() {
        return;
    }
    let Some(mut h) = world.get_mut::<Health>(target) else {
        return;
    };
    if !h.alive() {
        return;
    }
    h.hp -= req.amount.max(0);
    let dead = !h.alive();
    drop(h);
    if dead {
        remove_defender(world, target, RemoveCause::Killed);
    }
}

/// SPEC 6 第 12 步：销毁本 tick 死亡的敌人（tick 末生效）；Carrier 在原地立即生成衍生敌人。
pub fn death_system(world: &mut World) {
    let deaths = std::mem::take(&mut world.resources.expect_mut::<DeathLog>().0);
    for d in deaths {
        world.commands.despawn(d.entity);
        let child = world.get::<Carrier>(d.entity).map(|c| c.child);
        if d.spawn_children
            && let Some(child) = child
        {
            spawn_enemy(world, child, d.lane, d.x, None);
        }
    }
}

/// SPEC 6 第 13 步：敌人越过失败线时，该车道 Hose Burst 可用则清场（全部敌人即死，不衍生），
/// 否则 `phase = Lost` 并发 `BattleEnded(Lost)`。
pub fn lane_defense_system(world: &mut World) {
    let mut crossed: Vec<(Entity, u8)> = world
        .query::<(Entity, &EnemyRef, &Health, &Lane, &Transform2D)>()
        .iter()
        .filter(|(_, _, h, _, t)| h.alive() && t.pos.x < Grid::FAIL_X)
        .map(|(e, _, _, l, _)| (e, l.row))
        .collect();
    crossed.sort_by_key(|(e, _)| e.to_bits());
    for (e, row) in crossed {
        if !world.get::<Health>(e).is_some_and(|h| h.alive()) {
            continue;
        }
        let lanes = world.resources.expect_mut::<LaneState>();
        let Some(slot) = lanes.hose_available.get_mut(usize::from(row)) else {
            continue;
        };
        if !*slot {
            world.resources.expect_mut::<BattleState>().phase = Phase::Lost;
            world.events.send(BattleEnded {
                outcome: Outcome::Lost,
            });
            return;
        }
        *slot = false;
        world.events.send(HoseTriggered { lane: row });
        let victims: Vec<Entity> = world
            .query::<(Entity, &EnemyRef, &Health, &Lane)>()
            .iter()
            .filter(|(_, _, h, l)| h.alive() && l.row == row)
            .map(|(v, _, _, _)| v)
            .collect();
        for (seq, target) in (0..).zip(victims) {
            let req = DamageRequest {
                target,
                amount: 0,
                source: DamageSource::Hose,
                ignore_armor: true,
                seq,
            };
            damage_enemy(world, req);
        }
        death_system(world);
    }
}
