//! 规则测试的公共夹具：从 `assets/data/` 读取真实数据（不经 content 校验），
//! 以 `App::step_ticks` 驱动 `battle_tick`。事件在 `step_ticks` 下不翻页，测试可读取全部历史事件。

#![allow(dead_code)]

use std::path::Path;

use rootguard_core::battle::{defender_bundle, spawn_enemy};
use rootguard_core::components::{Cell, Health, Shielded, Walker};
use rootguard_core::data::{CardRule, LaneSpec, LevelDef, SpawnDef, WaveDef, parse_ron};
use rootguard_core::registry::{
    Content, DefenderRegistry, EnemyRegistry, LevelRegistry, ProjectileRegistry,
};
use rootguard_core::resources::{BattleState, Phase};
use rootguard_core::{DefenderId, EnemyId, LevelId, add_core_plugins, battle_tick, start_battle};
use trellia_core::{App, Entity, Event, Stage, Transform2D, Vec2, World};

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data")
}

fn read_all<T: serde::de::DeserializeOwned>(sub: &str) -> Vec<T> {
    let dir = data_dir().join(sub);
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .expect("数据目录")
        .map(|e| e.expect("目录项").path())
        .filter(|p| p.extension().is_some_and(|e| e == "ron"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("读取");
            parse_ron(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
        })
        .collect()
}

pub fn did(s: &str) -> DefenderId {
    DefenderId::new(s).expect("id")
}

pub fn eid(s: &str) -> EnemyId {
    EnemyId::new(s).expect("id")
}

pub fn lid(s: &str) -> LevelId {
    LevelId::new(s).expect("id")
}

/// 测试关卡：五车道、无天空 Dew、Dew 充足、一波极晚出现（测试手动生成敌人）。
pub fn test_level(waves: Vec<WaveDef>) -> LevelDef {
    LevelDef {
        id: lid("0-1"),
        seed: 7,
        lanes: vec![0, 1, 2, 3, 4],
        start_dew: 5000,
        sky_dew: false,
        card_slots: 8,
        cards: CardRule::Choose,
        unlock_on_win: vec![],
        waves,
    }
}

pub fn far_wave() -> Vec<WaveDef> {
    vec![WaveDef {
        delay_ticks: 1_000_000,
        flag: true,
        spawns: vec![SpawnDef {
            enemy: eid("husk"),
            count: 1,
            lane: LaneSpec::Fixed(0),
        }],
    }]
}

pub fn content_with(levels: Vec<LevelDef>) -> Content {
    let mut defenders = DefenderRegistry::default();
    for d in read_all::<rootguard_core::data::DefenderDef>("defenders") {
        defenders.insert(d.id, d);
    }
    let mut projectiles = ProjectileRegistry::default();
    for p in read_all::<rootguard_core::data::ProjectileDef>("projectiles") {
        projectiles.insert(p.id, p);
    }
    let mut enemies = EnemyRegistry::default();
    for e in read_all::<rootguard_core::data::EnemyDef>("enemies") {
        enemies.insert(e.id, e);
    }
    let mut all_levels = read_all::<LevelDef>("levels");
    all_levels.extend(levels);
    let rules_text = std::fs::read_to_string(data_dir().join("rules.ron")).expect("rules.ron");
    Content {
        defenders,
        projectiles,
        enemies,
        levels: LevelRegistry::new(all_levels),
        rules: parse_ron(&rules_text).expect("rules.ron 解析"),
    }
}

/// 对局夹具。
pub struct Sim {
    pub app: App,
}

impl Sim {
    /// 测试关卡 + 指定卡组，直接进入 `Playing`。
    pub fn new(cards: &[&str]) -> Self {
        Self::with_level(test_level(far_wave()), cards)
    }

    pub fn with_level(level: LevelDef, cards: &[&str]) -> Self {
        let id = level.id;
        let mut app = App::new();
        add_core_plugins(&mut app, content_with(vec![level]));
        app.add_system(Stage::FixedUpdate, battle_tick);
        let cards: Vec<DefenderId> = cards.iter().map(|c| did(c)).collect();
        start_battle(app.world_mut(), id, &cards).expect("开局");
        app.world_mut().resources.expect_mut::<BattleState>().phase = Phase::Playing;
        Self { app }
    }

    pub fn world(&self) -> &World {
        self.app.world()
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }

    pub fn step(&mut self, n: u32) {
        self.app.step_ticks(n);
    }

    /// 立即生成敌人（`x` 为逻辑坐标）。
    pub fn enemy(&mut self, id: &str, row: u8, x: f32) -> Entity {
        spawn_enemy(self.world_mut(), eid(id), row, x, None).expect("敌人 id")
    }

    /// 生成静止的敌人（基础速度置 0）。
    pub fn frozen_enemy(&mut self, id: &str, row: u8, x: f32) -> Entity {
        let e = self.enemy(id, row, x);
        self.world_mut()
            .get_mut::<Walker>(e)
            .expect("Walker")
            .base_speed = 0.0;
        e
    }

    /// 绕过放置规则立即生成防御者（下一 tick 起参与模拟）。
    pub fn defender(&mut self, id: &str, row: u8, col: u8) -> Entity {
        let def = self
            .world()
            .resources
            .expect::<DefenderRegistry>()
            .get(did(id))
            .expect("防御者 id")
            .clone();
        let mut b = defender_bundle(&def, Cell { row, col }, None);
        self.world_mut().spawn(b.build())
    }

    /// 在宿主上立即生成覆盖层。
    pub fn overlay(&mut self, id: &str, host: Entity) -> Entity {
        let def = self
            .world()
            .resources
            .expect::<DefenderRegistry>()
            .get(did(id))
            .expect("防御者 id")
            .clone();
        let cell = *self.world().get::<Cell>(host).expect("宿主格");
        let mut b = defender_bundle(&def, cell, Some(host));
        let e = self.world_mut().spawn(b.build());
        self.world_mut()
            .entities
            .insert_one(host, Shielded { overlay: e })
            .expect("宿主存在");
        e
    }

    pub fn hp(&self, e: Entity) -> i32 {
        self.world().get::<Health>(e).map_or(0, |h| h.hp)
    }

    pub fn x(&self, e: Entity) -> f32 {
        self.world()
            .get::<Transform2D>(e)
            .expect("Transform2D")
            .pos
            .x
    }

    pub fn teleport(&mut self, e: Entity, x: f32) {
        let mut t = self
            .world_mut()
            .get_mut::<Transform2D>(e)
            .expect("Transform2D");
        let y = t.pos.y;
        t.teleport(Vec2::new(x, y));
    }

    pub fn alive(&self, e: Entity) -> bool {
        self.world().contains(e) && self.hp(e) > 0
    }

    /// 全部历史事件（`step_ticks` 下事件不翻页）。
    pub fn events<T: Event + Clone>(&self) -> Vec<T> {
        self.world().events.get::<T>().iter().cloned().collect()
    }

    pub fn state(&self) -> BattleState {
        self.world().resources.expect::<BattleState>().clone()
    }
}

/// 第 `col` 列中心 x。
pub fn col_x(col: u8) -> f32 {
    rootguard_core::Grid::cell_center(0, col).x
}
