//! 录制脚本解用的启发式机器人（只在录制时使用；回放测试只读 RON 时间表）。
//!
//! 每 tick 读取模拟状态，产出对局 `UiAction`：收集全部 Dew，然后至多放置一张卡。
//! 优先级：紧急处置 → 经济 → 射手 → 墙 → 地雷 / 地刺 → 覆盖层。

use rootguard_core::Grid;
use rootguard_core::components::{
    Armor, Cell, DefenderRef, EnemyRef, Health, Heavy, Lane, OverlayOf,
};
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

/// 第一株霜籽荚对车道的额外价值（减速）；同车道的第二株不再计入。
const FROST_BONUS: f32 = 0.6;

fn is_frost(def: &DefenderDef) -> bool {
    matches!(
        def.behaviors.first(),
        Some(Behavior::Shooter { projectile, .. }) if projectile.as_str().contains("frost")
    )
}

/// 射手火力（每 85 tick 的伤害次数，近似）。
fn shooter_power(def: &DefenderDef) -> f32 {
    match def.behaviors.first() {
        Some(Behavior::Shooter { volley, .. }) => {
            f32::from(*volley) + if is_frost(def) { FROST_BONUS } else { 0.0 }
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
    /// 墙优先于经济与射手（墙便宜且冷却长，有敌人来袭的车道一就绪就放）。
    pub wall_first: bool,
    /// 射手分配时敌人生命的权重（每点火力抵消的生命）；越小越均衡。
    pub focus: f32,
    /// 补第二排生产者前，每条车道至少要有的射手数。
    pub min_shooters: usize,
    /// 埋地雷的最低敌群生命。
    pub mine_min_hp: i32,
    /// 放炽球的最低 3 × 3 敌人生命。
    pub blast_min_hp: i32,
    /// 咬夹草的目标最低生命。
    pub chomp_min_hp: i32,
    /// 需要射手但 Dew 不够时攒钱（不把钱花在低优先级上）。
    pub save_for_shooters: bool,
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
            wall_first: false,
            focus: 12.0,
            min_shooters: 1,
            mine_min_hp: 20,
            blast_min_hp: 80,
            chomp_min_hp: 17,
            save_for_shooters: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct EnemyInfo {
    row: u8,
    x: f32,
    hp: i32,
    heavy: bool,
}

#[derive(Clone, Copy, Debug)]
struct DefInfo {
    cell: Cell,
    role: Role,
    frost: bool,
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
        // 冷却已好、只差 Dew 的射手：需要时应攒钱而不是把钱花在低优先级上
        let shooter_saving = tray.cards.iter().any(|c| {
            !c.ready
                && c.cooldown.finished()
                && defs
                    .get(c.defender)
                    .is_some_and(|d| role(d) == Role::Shooter)
        });
        if ready.is_empty() {
            return None;
        }
        let enemies: Vec<EnemyInfo> = world
            .query::<(
                &EnemyRef,
                &Health,
                &Lane,
                &Transform2D,
                Option<&Armor>,
                Option<&Heavy>,
            )>()
            .iter()
            .filter(|(_, h, _, _, _, _)| h.alive())
            .map(|(_, h, l, t, a, heavy)| EnemyInfo {
                row: l.row,
                x: t.pos.x,
                hp: h.hp + a.map_or(0, |a| a.hp.max(0)),
                heavy: heavy.is_some(),
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
                    frost: is_frost(def),
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
        let lane_pressure = |row: u8| lane_hp(row) as f32 - lane_power(row) * self.cfg.focus;
        // 安全放置：射手 / 生产者只放在最左敌人所在列再往左两列以内，避免刚放下就被啃
        let safe_limit = |row: u8| -> u8 {
            match Grid::col_at_x(lane_min_x(row)) {
                Some(ecol) => ecol.saturating_sub(1),
                None => Grid::COLS,
            }
        };

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

        // 墙：有敌人来袭、已有火力、该车道尚无健康的墙；放在射手右侧、敌人前方
        let wall_choice = || -> Option<(usize, u8, u8)> {
            let (slot, _) = first(Role::Wall)?;
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
            order.sort_by(|a, b| lane_min_x(*a).total_cmp(&lane_min_x(*b)));
            for row in order {
                let ecol = Grid::col_at_x(lane_min_x(row)).unwrap_or(Grid::COLS - 1);
                let back = placed
                    .iter()
                    .filter(|d| d.cell.row == row && d.role != Role::Wall)
                    .map(|d| d.cell.col + 1)
                    .max()
                    .unwrap_or(1);
                let front = ecol.saturating_sub(1).min(self.cfg.wall_col + 1);
                for col in (back..=front).rev() {
                    if can(slot, row, col) {
                        return Some((slot, row, col));
                    }
                }
            }
            None
        };

        // 性价比最高的就绪射手（空间紧张时改为火力最高）
        let shooters: Vec<(usize, &DefenderDef)> = ready
            .iter()
            .filter(|(_, r, _)| *r == Role::Shooter)
            .map(|(i, _, d)| (*i, *d))
            .collect();
        let frost_in = |row: u8| placed.iter().any(|d| d.cell.row == row && d.frost);
        let free_cols = |row: u8| {
            (0..self.cfg.wall_col)
                .filter(|&c| !placed.iter().any(|d| d.cell.row == row && d.cell.col == c))
                .count()
        };
        let pick_shooter = |row: u8| -> Option<usize> {
            let value = |d: &DefenderDef| {
                let mut p = shooter_power(d);
                if is_frost(d) && frost_in(row) {
                    p -= FROST_BONUS;
                }
                if free_cols(row) <= 2 {
                    p
                } else {
                    p / d.cost.max(1) as f32
                }
            };
            shooters
                .iter()
                .max_by(|a, b| value(a.1).total_cmp(&value(b.1)).then(b.0.cmp(&a.0)))
                .map(|(i, _)| *i)
        };
        let place_shooter = |row: u8| -> Option<(usize, u8, u8)> {
            let slot = pick_shooter(row)?;
            (0..self.cfg.wall_col.min(safe_limit(row)))
                .find(|&c| can(slot, row, c))
                .map(|c| (slot, row, c))
        };
        // 车道缺口：敌人生命超过现有火力可承担的部分（火力为 0 时任何来敌都算缺口）
        let deficit = |row: u8| lane_hp(row) as f32 - lane_power(row) * self.cfg.focus;

        // 2. 有缺口的车道：先补射手，射手已有则补墙；按敌人远近排序
        let mut short: Vec<u8> = lanes
            .iter()
            .copied()
            .filter(|&r| lane_hp(r) > 0 && (deficit(r) > 0.0 || lane_power(r) == 0.0))
            .collect();
        short.sort_by(|a, b| lane_min_x(*a).total_cmp(&lane_min_x(*b)));
        for &row in &short {
            if let Some(choice) = place_shooter(row) {
                return Some(choice);
            }
        }
        let short_has_room = short.iter().any(|&r| {
            (0..self.cfg.wall_col.min(safe_limit(r)))
                .any(|c| !placed.iter().any(|d| d.cell.row == r && d.cell.col == c))
        });
        if self.cfg.wall_first
            && let Some(choice) = wall_choice()
        {
            return Some(choice);
        }
        // 缺口车道需要射手但钱不够：攒钱（墙已在上面处理）
        if self.cfg.save_for_shooters && short_has_room && shooters.is_empty() && shooter_saving {
            if let Some(choice) = wall_choice() {
                return Some(choice);
            }
            return None;
        }

        // 2b. 战术单位：便宜的地雷、对付密集敌群的炽球、对付高护甲的咬夹草
        if let Some(choice) = self.tactics(world, &ready, &enemies, &placed, &lanes) {
            return Some(choice);
        }

        // 3. 经济：第一排生产者；每条车道都有足够射手后再补到 producers_per_lane
        let producer = |target: usize| -> Option<(usize, u8, u8)> {
            if tick >= self.cfg.econ_until {
                return None;
            }
            let (slot, _) = first(Role::Producer)?;
            let mut order: Vec<u8> = lanes
                .iter()
                .copied()
                .filter(|&r| count(r, Role::Producer) < target)
                .collect();
            order.sort_by_key(|&r| (count(r, Role::Producer), r));
            for row in order {
                let limit = (target as u8 + 1).min(safe_limit(row));
                if let Some(col) = (0..limit).find(|&c| can(slot, row, c)) {
                    return Some((slot, row, col));
                }
            }
            None
        };
        if let Some(choice) = producer(1) {
            return Some(choice);
        }
        let mut thin: Vec<u8> = lanes
            .iter()
            .copied()
            .filter(|&r| count(r, Role::Shooter) < self.cfg.min_shooters)
            .collect();
        thin.sort_by(|a, b| lane_min_x(*a).total_cmp(&lane_min_x(*b)).then(a.cmp(b)));
        for &row in &thin {
            if let Some(choice) = place_shooter(row) {
                return Some(choice);
            }
        }
        if self.cfg.save_for_shooters && !thin.is_empty() && shooters.is_empty() && shooter_saving {
            return None;
        }
        if let Some(choice) = producer(self.cfg.producers_per_lane) {
            return Some(choice);
        }

        // 4. 射手：没有缺口时均衡补强（火力最低的车道优先，同等时敌人多者优先）
        if !shooters.is_empty() {
            let mut order: Vec<u8> = lanes
                .iter()
                .copied()
                .filter(|&r| count(r, Role::Shooter) < self.cfg.max_shooters_per_lane)
                .collect();
            order.sort_by(|a, b| {
                deficit(*b)
                    .total_cmp(&deficit(*a))
                    .then(lane_power(*a).total_cmp(&lane_power(*b)))
                    .then(a.cmp(b))
            });
            for row in order {
                if let Some(choice) = place_shooter(row) {
                    return Some(choice);
                }
            }
        }

        // 5. 墙
        if let Some(choice) = wall_choice() {
            return Some(choice);
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

        // 7. 地刺：放在墙的右侧一格，啃墙的敌人站在这一格上
        if let Some((slot, _)) = first(Role::Spikes) {
            let mut walls: Vec<&DefInfo> = placed.iter().filter(|d| d.role == Role::Wall).collect();
            walls.sort_by_key(|w| std::cmp::Reverse(lane_hp(w.cell.row)));
            for w in walls {
                let col = w.cell.col + 1;
                if col < Grid::COLS && lane_hp(w.cell.row) > 0 && can(slot, w.cell.row, col) {
                    return Some((slot, w.cell.row, col));
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

    /// 战术单位（地雷、炽球、咬夹草）。
    fn tactics(
        &self,
        world: &World,
        ready: &[(usize, Role, &DefenderDef)],
        enemies: &[EnemyInfo],
        placed: &[DefInfo],
        lanes: &[u8],
    ) -> Option<(usize, u8, u8)> {
        let can = |slot: usize, row: u8, col: u8| {
            check_placement(world, slot, Cell { row, col }).is_some()
        };
        let first = |r: Role| {
            ready
                .iter()
                .find(|(_, role, _)| *role == r)
                .map(|(i, _, _)| *i)
        };
        let col_of = |x: f32| Grid::col_at_x(x).unwrap_or(Grid::COLS - 1);
        let occupied = |row: u8, col: u8| {
            placed
                .iter()
                .any(|d| d.cell.row == row && d.cell.col == col)
        };
        // 地雷：敌群（≥ mine_min_hp）还在右侧时，埋在其前方约 3 格（武装需要时间）
        if let Some(slot) = first(Role::Mine) {
            let mut best: Option<(i32, u8, u8)> = None;
            for &row in lanes {
                let lead = enemies
                    .iter()
                    .filter(|e| e.row == row)
                    .map(|e| e.x)
                    .fold(f32::INFINITY, f32::min);
                if !lead.is_finite() {
                    continue;
                }
                let ecol = col_of(lead);
                let group: i32 = enemies
                    .iter()
                    .filter(|e| e.row == row && e.x < lead + Grid::CELL_SIZE.x)
                    .map(|e| e.hp)
                    .sum();
                if group < self.cfg.mine_min_hp || ecol < 5 {
                    continue;
                }
                let col = ecol - 3;
                if can(slot, row, col) && best.is_none_or(|(g, _, _)| group > g) {
                    best = Some((group, row, col));
                }
            }
            if let Some((_, row, col)) = best {
                return Some((slot, row, col));
            }
        }
        // 炽球：3 × 3 内敌人生命 ≥ blast_min_hp
        if let Some(slot) = first(Role::Blast) {
            let mut best: Option<(i32, u8, u8)> = None;
            for &row in lanes {
                for col in 1..Grid::COLS {
                    if occupied(row, col) {
                        continue;
                    }
                    let hp: i32 = enemies
                        .iter()
                        .filter(|e| {
                            e.row.abs_diff(row) <= 1 && Grid::enemy_overlaps_cols(e.x, col, 1, 1)
                        })
                        .map(|e| e.hp)
                        .sum();
                    if hp >= self.cfg.blast_min_hp
                        && can(slot, row, col)
                        && best.is_none_or(|(h, _, _)| hp > h)
                    {
                        best = Some((hp, row, col));
                    }
                }
            }
            if let Some((_, row, col)) = best {
                return Some((slot, row, col));
            }
        }
        // 咬夹草：车道里有高生命的非重型敌人时，放在其前方两格
        if let Some(slot) = first(Role::Chomper) {
            for &row in lanes {
                let big = enemies
                    .iter()
                    .filter(|e| e.row == row && e.hp >= self.cfg.chomp_min_hp && !e.heavy)
                    .map(|e| e.x)
                    .fold(f32::INFINITY, f32::min);
                if !big.is_finite() {
                    continue;
                }
                let ecol = col_of(big);
                if ecol >= 3 {
                    let col = ecol - 2;
                    if can(slot, row, col) {
                        return Some((slot, row, col));
                    }
                }
            }
        }
        None
    }
}
