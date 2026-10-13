//! SPEC 4.5 校验规则。全部错误一次性收集，每条含文件路径与字段名。

use std::collections::BTreeMap;

use rootguard_core::Grid;
use rootguard_core::data::{Behavior, CardRule, LaneSpec, Trait};
use rootguard_core::ids::{DefenderId, EnemyId, ProjectileId};

use crate::{RawContent, ValidationError};

/// 卡槽数允许范围（SPEC 4.5）。
const CARD_SLOTS: std::ops::RangeInclusive<u8> = 4..=8;

struct Errors(Vec<ValidationError>);

impl Errors {
    fn push(&mut self, path: &str, field: impl Into<String>, reason: impl Into<String>) {
        self.0.push(ValidationError {
            path: path.to_owned(),
            field: field.into(),
            reason: reason.into(),
        });
    }

    fn check(&mut self, ok: bool, path: &str, field: impl Into<String>, reason: &str) {
        if !ok {
            self.push(path, field, reason);
        }
    }
}

/// 内容 id 只含 `[a-z0-9_]` 且非空（SPEC 4.5）。
fn valid_content_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// 关卡 id 为 `章-序`（两段十进制数字，ADR-002）。
fn valid_level_id(s: &str) -> bool {
    let mut parts = s.split('-');
    let num =
        |p: Option<&str>| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    num(parts.next()) && num(parts.next()) && parts.next().is_none()
}

/// 执行 SPEC 4.5 的全部校验，返回错误列表（空表示通过）。
#[must_use]
pub fn validate(raw: &RawContent) -> Vec<ValidationError> {
    let mut e = Errors(Vec::new());

    // id 字符集与全局唯一（防御者 / 弹丸 / 敌人共享命名空间，关卡单独）
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let ids = raw
        .defenders
        .iter()
        .map(|d| (d.path.as_str(), d.def.id.as_str()))
        .chain(
            raw.projectiles
                .iter()
                .map(|p| (p.path.as_str(), p.def.id.as_str())),
        )
        .chain(
            raw.enemies
                .iter()
                .map(|x| (x.path.as_str(), x.def.id.as_str())),
        );
    for (path, id) in ids {
        e.check(valid_content_id(id), path, "id", "只允许 [a-z0-9_]");
        if let Some(first) = seen.insert(id.to_owned(), path.to_owned()) {
            e.push(path, "id", format!("id `{id}` 与 {first} 重复"));
        }
    }
    let mut level_seen: BTreeMap<String, String> = BTreeMap::new();
    for l in &raw.levels {
        let id = l.def.id.as_str();
        e.check(valid_level_id(id), &l.path, "id", "关卡 id 必须形如 `1-3`");
        if let Some(first) = level_seen.insert(id.to_owned(), l.path.clone()) {
            e.push(&l.path, "id", format!("关卡 id `{id}` 与 {first} 重复"));
        }
    }

    let defender = |id: DefenderId| raw.defenders.iter().any(|d| d.def.id == id);
    let projectile = |id: ProjectileId| raw.projectiles.iter().any(|p| p.def.id == id);
    let enemy = |id: EnemyId| raw.enemies.iter().any(|x| x.def.id == id);

    for d in &raw.defenders {
        let (p, def) = (d.path.as_str(), &d.def);
        e.check(def.hp >= 1, p, "hp", "必须 ≥ 1");
        e.check(def.cost >= 0, p, "cost", "必须 ≥ 0");
        e.check(def.cooldown_ticks >= 1, p, "cooldown_ticks", "必须 ≥ 1");
        e.check(!def.behaviors.is_empty(), p, "behaviors", "不能为空");
        for (i, b) in def.behaviors.iter().enumerate() {
            let f = |name: &str| format!("behaviors[{i}].{name}");
            match *b {
                Behavior::Shooter {
                    interval_ticks,
                    volley,
                    projectile: pid,
                    ..
                } => {
                    e.check(interval_ticks >= 1, p, f("interval_ticks"), "必须 ≥ 1");
                    e.check(volley >= 1, p, f("volley"), "必须 ≥ 1");
                    e.check(projectile(pid), p, f("projectile"), "引用的弹丸不存在");
                }
                Behavior::Producer {
                    interval_ticks,
                    amount,
                    ..
                } => {
                    e.check(interval_ticks >= 1, p, f("interval_ticks"), "必须 ≥ 1");
                    e.check(amount >= 0, p, f("amount"), "必须 ≥ 0");
                }
                Behavior::Spikes { interval_ticks, .. } => {
                    e.check(interval_ticks >= 1, p, f("interval_ticks"), "必须 ≥ 1");
                }
                Behavior::Overlay {
                    repair_below_num,
                    repair_below_den,
                } => {
                    e.check(repair_below_den >= 1, p, f("repair_below_den"), "必须 ≥ 1");
                    e.check(repair_below_num >= 0, p, f("repair_below_num"), "必须 ≥ 0");
                }
                Behavior::Gust { push_px, .. } => {
                    e.check(push_px > 0.0, p, f("push_px"), "必须 > 0");
                }
                Behavior::Wall
                | Behavior::Mine { .. }
                | Behavior::Blast { .. }
                | Behavior::Chomper { .. } => {}
            }
        }
    }

    for x in &raw.projectiles {
        e.check(x.def.speed > 0.0, &x.path, "speed", "必须 > 0");
        e.check(x.def.damage >= 0, &x.path, "damage", "必须 ≥ 0");
    }

    for x in &raw.enemies {
        let (p, def) = (x.path.as_str(), &x.def);
        e.check(def.hp >= 1, p, "hp", "必须 ≥ 1");
        e.check(def.speed > 0.0, p, "speed", "必须 > 0");
        e.check(
            def.bite_interval_ticks >= 1,
            p,
            "bite_interval_ticks",
            "必须 ≥ 1",
        );
        if let Some(armor) = def.armor {
            e.check(armor.split().1 >= 1, p, "armor.hp", "必须 ≥ 1");
        }
        for (i, t) in def.traits.iter().enumerate() {
            if let Trait::Carrier { child } = *t {
                e.check(
                    enemy(child),
                    p,
                    format!("traits[{i}].child"),
                    "引用的敌人不存在",
                );
                e.check(
                    child != def.id,
                    p,
                    format!("traits[{i}].child"),
                    "不能衍生自身",
                );
            }
        }
    }

    for l in &raw.levels {
        let (p, def) = (l.path.as_str(), &l.def);
        e.check(!def.lanes.is_empty(), p, "lanes", "不能为空");
        for (i, &row) in def.lanes.iter().enumerate() {
            e.check(row < Grid::ROWS, p, format!("lanes[{i}]"), "车道超出 0..5");
            e.check(
                !def.lanes[..i].contains(&row),
                p,
                format!("lanes[{i}]"),
                "车道重复",
            );
        }
        e.check(def.start_dew >= 0, p, "start_dew", "必须 ≥ 0");
        e.check(
            CARD_SLOTS.contains(&def.card_slots),
            p,
            "card_slots",
            "必须在 [4, 8] 内",
        );
        if let CardRule::Forced(list) = &def.cards {
            e.check(
                list.len() <= usize::from(def.card_slots),
                p,
                "cards",
                "Forced 列表长度必须 ≤ card_slots",
            );
            e.check(!list.is_empty(), p, "cards", "Forced 列表不能为空");
            for (i, &id) in list.iter().enumerate() {
                e.check(defender(id), p, format!("cards[{i}]"), "引用的卡牌不存在");
            }
        }
        for (i, &id) in def.unlock_on_win.iter().enumerate() {
            e.check(
                defender(id),
                p,
                format!("unlock_on_win[{i}]"),
                "引用的卡牌不存在",
            );
        }
        e.check(!def.waves.is_empty(), p, "waves", "不能为空");
        e.check(
            def.waves.iter().any(|w| w.flag),
            p,
            "waves",
            "至少需要 1 个 flag 波",
        );
        e.check(
            def.waves.last().is_none_or(|w| w.flag),
            p,
            "waves",
            "最后一波必须为 flag 波",
        );
        for (wi, w) in def.waves.iter().enumerate() {
            e.check(
                !w.spawns.is_empty(),
                p,
                format!("waves[{wi}].spawns"),
                "不能为空",
            );
            for (si, s) in w.spawns.iter().enumerate() {
                let f = |name: &str| format!("waves[{wi}].spawns[{si}].{name}");
                e.check(enemy(s.enemy), p, f("enemy"), "引用的敌人不存在");
                e.check(s.count >= 1, p, f("count"), "必须 ≥ 1");
                if let LaneSpec::Fixed(row) = s.lane {
                    e.check(
                        def.lanes.contains(&row),
                        p,
                        f("lane"),
                        "Fixed 车道必须在 lanes 内",
                    );
                }
            }
        }
    }

    e.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_formats() {
        assert!(valid_content_id("pebble_pod"));
        assert!(!valid_content_id("Pebble"));
        assert!(!valid_content_id("a-b"));
        assert!(valid_level_id("1-10"));
        assert!(!valid_level_id("1-"));
        assert!(!valid_level_id("1-2-3"));
        assert!(!valid_level_id("a-1"));
    }
}
