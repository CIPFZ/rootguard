//! 录制脚本解用的启发式机器人（只在录制时使用；回放测试只读 RON 时间表）。
//!
//! 每 tick 读取模拟状态，产出对局 `UiAction`：收集全部 Dew，然后至多放置一张卡。
//! 优先级：紧急处置 → 经济 → 射手 → 墙 → 地雷 / 地刺 → 覆盖层。

use rootguard_core::Grid;
use rootguard_core::components::{Armor, Cell, DefenderRef, EnemyRef, Health, Lane, OverlayOf};
use rootguard_core::data::{Behavior, DefenderDef};
use rootguard_core::events::UiAction;
use rootguard_core::registry::{DefenderRegistry, EnemyRegistry};
use rootguard_core::resources::{BattleState, CardTray, LaneState, Phase};
use rootguard_core::systems::check_placement;
use trellia::prelude::*;

use super::dew_positions;

/// 卡牌角色。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Producer,
    Shooter,
    Wall,
    Spikes,
    Mine,
    Blast,
    Shell,
    Chomper,
    Gust,
}

fn role(def: &DefenderDef) -> Role {
    match def.behaviors.first() {
        Some(Behavior::Producer { .. }) => Role::Producer,
        Some(Behavior::Shooter { .. }) => Role::Shooter,
        Some(Behavior::Spikes { .. }) => Role::Spikes,
        Some(Behavior::Mine { .. }) => Role::Mine,
        Some(Behavior::Blast { .. }) => Role::Blast,
        Some(Behavior::Overlay { .. }) => Role::Shell,
        Some(Behavior::Chomper { .. }) => Role::Chomper,
        Some(Behavior::Gust { .. }) => Role::Gust,
        Some(Behavior::Wall) | None => Role::Wall,
    }
}

/// 射手火力（每 85 tick 的伤害次数，近似）。
fn shooter_power(def: &DefenderDef) -> f32 {
    match def.behaviors.first() {
        Some(Behavior::Shooter {
            volley, projectile, ..
        }) => {
            f32::from(*volley)
                + if projectile.as_str().contains("frost") {
                    0.6
                } else {
                    0.0
                }
        }
        _ => 0.0,
    }
}

/// 机器人参数。
#[derive(Clone, Debug)]
pub struct BotConfig {
    /// 每条激活车道的目标生产者数。
    pub producers_per_lane: usize,
    /// 停止新建生产者的 `level_tick`。
    pub econ_until: u64,
    /// 每条车道射手上限。
    pub max_shooters_per_lane: usize,
    /// 紧急处置线：敌人 x 小于此值。
    pub emergency_x: f32,
    /// 墙放置的列。
    pub wall_col: u8,
    /// 开始放墙的最低射手火力。
    pub wall_min_power: f32,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            producers_per_lane: 1,
            econ_until: 6000,
            max_shooters_per_lane: 6,
            emergency_x: 560.0,
            wall_col: 6,
            wall_min_power: 2.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct EnemyInfo {
    row: u8,
    x: f32,
    hp: i32,
}

#[derive(Clone, Copy, Debug)]
struct DefInfo {
    cell: Cell,
    role: Role,
    power: f32,
    hp: i32,
    max: i32,
}

/// 机器人。
pub struct Bot {
    pub cfg: BotConfig,
}

impl Bot {
    pub fn new(cfg: BotConfig) -> Self {
        Self { cfg }
    }

    /// 本 tick 的动作。
    pub fn decide(&mut self, world: &World) -> Vec<UiAction> {
        let Some(state) = world.resources.get::<BattleState>() else {
            return Vec::new();
        };
        if state.phase != Phase::Playing {
            return Vec::new();
        }
        let mut out: Vec<UiAction> = dew_positions(world)
            .into_iter()
            .map(UiAction::CollectDewAt)
            .collect();
        if let Some((slot, row, col)) = self.choose(world, state.level_tick) {
            out.push(UiAction::SelectCard(slot));
            out.push(UiAction::PlaceAt { row, col });
        }
        out
    }

    fn choose(&self, world: &World, tick: u64) -> Option<(usize, u8, u8)> {
        let defs = world.resources.get::<DefenderRegistry>()?;
        let _enemy_defs = world.resources.get::<EnemyRegistry>()?;
        let tray = world.resources.get::<CardTray>()?;
        let lanes = world.resources.get::<LaneState>()?.active_rows();
        let ready: Vec<(usize, Role, &DefenderDef)> = tray
            .cards
            .iter()
            .enumerate()
            .filter(|(_, c)| c.ready)
            .filter_map(|(i, c)| defs.get(c.defender).map(|d| (i, role(d), d)))
            .collect();
        if ready.is_empty() {
            return None;
        }
        let enemies: Vec<EnemyInfo> = world
            .query::<(&EnemyRef, &Health, &Lane, &Transform2D, Option<&Armor>)>()
            .iter()
            .filter(|(_, h, _, _, _)| h.alive())
            .map(|(_, h, l, t, a)| EnemyInfo {
                row: l.row,
                x: t.pos.x,
                hp: h.hp + a.map_or(0, |a| a.hp.max(0)),
            })
            .collect();
        let placed: Vec<DefInfo> = world
            .query::<(&DefenderRef, &Cell, &Health)>()
            .without::<&OverlayOf>()
            .iter()
            .filter(|(_, _, h)| h.alive())
            .filter_map(|(d, c, h)| {
                defs.get(d.id).map(|def| DefInfo {
                    cell: *c,
                    role: role(def),
                    power: shooter_power(def),
                    hp: h.hp,
                    max: h.max,
                })
            })
            .collect();
        let can = |slot: usize, row: u8, col: u8| {
            check_placement(world, slot, Cell { row, col }).is_some()
        };
        let first = |r: Role| {
            ready
                .iter()
                .find(|(_, role, _)| *role == r)
                .map(|(i, _, d)| (*i, *d))
        };
        let lane_hp = |row: u8| {
            enemies
                .iter()
                .filter(|e| e.row == row)
                .map(|e| e.hp)
                .sum::<i32>()
        };
        let lane_min_x = |row: u8| {
            enemies
                .iter()
                .filter(|e| e.row == row)
                .map(|e| e.x)
                .fold(f32::INFINITY, f32::min)
        };
        let lane_power = |row: u8| {
            placed
                .iter()
                .filter(|d| d.cell.row == row)
                .map(|d| d.power)
                .sum::<f32>()
        };
        let count = |row: u8, r: Role| {
            placed
                .iter()
                .filter(|d| d.cell.row == row && d.role == r)
                .count()
        };
        let lane_pressure = |row: u8| lane_hp(row) as f32 - lane_power(row) * 12.0;

        // 1. 紧急：敌人逼近且火力不足
        let mut urgent: Vec<u8> = lanes
            .iter()
            .copied()
            .filter(|&r| lane_min_x(r) < self.cfg.emergency_x && lane_pressure(r) > 0.0)
            .collect();
        urgent.sort_by(|a, b| lane_min_x(*a).total_cmp(&lane_min_x(*b)));
        for &row in &urgent {
            let x = lane_min_x(row);
            let ecol = Grid::col_at_x(x).unwrap_or(0);
            if let Some((slot, _)) = first(Role::Blast) {
                for col in [ecol, ecol.saturating_sub(1), (ecol + 1).min(Grid::COLS - 1)] {
                    if can(slot, row, col) {
                        return Some((slot, row, col));
                    }
                }
            }
            if let Some((slot, _)) = first(Role::Chomper) {
                let col = ecol.saturating_sub(1);
                if can(slot, row, col) {
                    return Some((slot, row, col));
                }
            }
            if let Some((slot, _)) = first(Role::Wall) {
                let col = ecol.saturating_sub(1);
                if can(slot, row, col) {
                    return Some((slot, row, col));
                }
            }
        }

        // 2. 有敌人却没有射手的车道先补一个射手
        let best_shooter = ready
            .iter()
            .filter(|(_, r, _)| *r == Role::Shooter)
            .max_by(|a, b| shooter_power(a.2).total_cmp(&shooter_power(b.2)))
            .map(|(i, _, _)| *i);
        if let Some(slot) = best_shooter {
            let mut bare: Vec<u8> = lanes
                .iter()
                .copied()
                .filter(|&r| lane_hp(r) > 0 && count(r, Role::Shooter) == 0)
                .collect();
            bare.sort_by(|a, b| lane_min_x(*a).total_cmp(&lane_min_x(*b)));
            for row in bare {
                if let Some(col) = (0..self.cfg.wall_col).find(|&c| can(slot, row, c)) {
                    return Some((slot, row, col));
                }
            }
        }

        // 3. 经济
        if tick < self.cfg.econ_until
            && let Some((slot, _)) = first(Role::Producer)
        {
            let target = self.cfg.producers_per_lane;
            let mut order: Vec<u8> = lanes
                .iter()
                .copied()
                .filter(|&r| count(r, Role::Producer) < target)
                .collect();
            order.sort_by_key(|&r| (count(r, Role::Producer), r));
            for row in order {
                if let Some(col) = (0..target as u8 + 1).find(|&c| can(slot, row, c)) {
                    return Some((slot, row, col));
                }
            }
        }

        // 4. 射手：火力最强的就绪射手放到压力最大的车道
        if let Some(slot) = best_shooter {
            let any_enemy = !enemies.is_empty();
            let mut order: Vec<u8> = lanes
                .iter()
                .copied()
                .filter(|&r| count(r, Role::Shooter) < self.cfg.max_shooters_per_lane)
                .collect();
            if any_enemy {
                order.sort_by(|a, b| {
                    lane_pressure(*b)
                        .total_cmp(&lane_pressure(*a))
                        .then(a.cmp(b))
                });
            } else {
                order.sort_by(|a, b| lane_power(*a).total_cmp(&lane_power(*b)).then(a.cmp(b)));
            }
            for row in order {
                if let Some(col) = (0..self.cfg.wall_col).find(|&c| can(slot, row, c)) {
                    return Some((slot, row, col));
                }
            }
        }

        // 5. 墙：有敌人来袭、已有火力、该车道右侧尚无墙
        if let Some((slot, _)) = first(Role::Wall) {
            let mut order: Vec<u8> = lanes
                .iter()
                .copied()
                .filter(|&r| {
                    lane_hp(r) > 0
                        && lane_power(r) >= self.cfg.wall_min_power
                        && !placed
                            .iter()
                            .any(|d| d.cell.row == r && d.role == Role::Wall && d.hp * 3 > d.max)
                })
                .collect();
            order.sort_by(|a, b| lane_pressure(*b).total_cmp(&lane_pressure(*a)));
            for row in order {
                for col in (self.cfg.wall_col..Grid::COLS - 1).chain((2..self.cfg.wall_col).rev()) {
                    if can(slot, row, col) {
                        return Some((slot, row, col));
                    }
                }
            }
        }

        // 6. 地雷：放在敌人最多车道的远端
        if let Some((slot, _)) = first(Role::Mine) {
            let mut order: Vec<u8> = lanes.iter().copied().filter(|&r| lane_hp(r) > 0).collect();
            order.sort_by(|a, b| lane_pressure(*b).total_cmp(&lane_pressure(*a)));
            for row in order {
                let x = lane_min_x(row);
                let ecol = Grid::col_at_x(x).unwrap_or(Grid::COLS - 1);
                if ecol >= 4
                    && let Some(col) = (3..ecol).rev().find(|&c| can(slot, row, c))
                {
                    return Some((slot, row, col));
                }
            }
        }

        // 7. 地刺
        if let Some((slot, _)) = first(Role::Spikes) {
            let mut order: Vec<u8> = lanes.iter().copied().filter(|&r| lane_hp(r) > 0).collect();
            order.sort_by(|a, b| lane_pressure(*b).total_cmp(&lane_pressure(*a)));
            for row in order {
                if let Some(col) = (self.cfg.wall_col + 1..Grid::COLS).find(|&c| can(slot, row, c))
                {
                    return Some((slot, row, col));
                }
            }
        }

        // 8. 覆盖层：受损的墙
        if let Some((slot, _)) = first(Role::Shell) {
            for d in placed.iter().filter(|d| d.role == Role::Wall) {
                if can(slot, d.cell.row, d.cell.col) {
                    return Some((slot, d.cell.row, d.cell.col));
                }
            }
        }

        // 9. 风推：某车道敌人密集且逼近
        if let Some((slot, _)) = first(Role::Gust) {
            for &row in &urgent {
                if let Some(col) = (0..Grid::COLS).find(|&c| can(slot, row, c)) {
                    return Some((slot, row, col));
                }
            }
        }
        None
    }
}
