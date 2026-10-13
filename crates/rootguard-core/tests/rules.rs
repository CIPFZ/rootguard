//! SPEC 16.1 规则单元测试（headless，无渲染）。每个测试名与 SPEC 表格一致。

mod common;

use common::{Sim, col_x, eid, lid, test_level};
use rootguard_core::components::{Armor, DewPickup, WalkState, Walker};
use rootguard_core::data::{LaneSpec, SpawnDef, WaveDef};
use rootguard_core::events::{
    BattleEnded, DamageSource, EnemyArmorBroken, EnemyDamaged, EnemyDied, EnemySpawned,
    HoseTriggered, Outcome, ProjectileHit, UiAction, WaveStarted,
};
use rootguard_core::grid::Grid;
use rootguard_core::resources::{BattleState, CardTray, DamageQueue, Phase, WaveState};
use rootguard_core::systems::damage_system;
use trellia_core::{Timer, Transform2D, Vec2};

const EPS: f32 = 1e-4;
/// 坐标在 ~1000 量级累加多次后的 f32 误差容限。
const POS_EPS: f32 = 1e-3;

fn hits_on(sim: &Sim, target: trellia_core::Entity) -> usize {
    sim.events::<ProjectileHit>()
        .iter()
        .filter(|h| h.target == target)
        .count()
}

fn died(sim: &Sim, e: trellia_core::Entity) -> bool {
    sim.events::<EnemyDied>().iter().any(|d| d.entity == e)
}

#[test]
fn pebble_kills_husk_in_10_hits() {
    let mut sim = Sim::new(&[]);
    sim.defender("pebble_pod", 2, 0);
    let husk = sim.frozen_enemy("husk", 2, col_x(6));
    let mut last_hits = 0;
    for _ in 0..2000 {
        sim.step(1);
        let hits = hits_on(&sim, husk);
        if died(&sim, husk) {
            assert_eq!(hits, 10, "第 10 发命中后死亡");
            assert_eq!(last_hits, 9, "死亡发生在第 10 发命中的同一 tick");
            return;
        }
        assert!(hits < 10);
        last_hits = hits;
    }
    panic!("2000 tick 内未击杀");
}

#[test]
fn clay_husk_armor_then_hp() {
    let mut sim = Sim::new(&[]);
    sim.defender("pebble_pod", 1, 0);
    let clay = sim.frozen_enemy("clay_husk", 1, col_x(7));
    let mut broken_at = None;
    for _ in 0..5000 {
        sim.step(1);
        let hits = hits_on(&sim, clay);
        if broken_at.is_none() && !sim.events::<EnemyArmorBroken>().is_empty() {
            broken_at = Some(hits);
            assert!(sim.world().get::<Armor>(clay).is_none(), "护甲破损后移除");
        }
        if died(&sim, clay) {
            assert_eq!(broken_at, Some(28), "第 28 发后护甲破损");
            assert_eq!(hits, 38, "28 + 10 = 38 发致死");
            return;
        }
    }
    panic!("5000 tick 内未击杀");
}

#[test]
fn frost_halves_speed_and_bite() {
    // 速度：霜籽命中后位移为基础一半
    let mut sim = Sim::new(&[]);
    sim.defender("frostpod", 1, 0);
    let husk = sim.enemy("husk", 1, col_x(6));
    while hits_on(&sim, husk) == 0 {
        sim.step(1);
    }
    let x0 = sim.x(husk);
    sim.step(10);
    assert!(
        ((x0 - sim.x(husk)) - 10.0 * 0.15).abs() < POS_EPS,
        "Chill 期间每 tick 0.15"
    );

    // 啃食：Chill 期间间隔 120
    let mut sim = Sim::new(&[]);
    let wall = sim.defender("barkwall", 3, 4);
    let husk = sim.enemy("husk", 3, Grid::cell_right(4) + Grid::ENEMY_HALF_WIDTH);
    sim.world_mut()
        .get_mut::<rootguard_core::components::StatusEffects>(husk)
        .expect("状态")
        .chill_ticks = 10_000;
    let mut drops = Vec::new();
    let mut last = sim.hp(wall);
    for tick in 1..=500 {
        sim.step(1);
        if sim.hp(wall) < last {
            drops.push(tick);
            last = sim.hp(wall);
        }
    }
    assert!(drops.len() >= 3, "{drops:?}");
    for pair in drops.windows(2) {
        assert_eq!(pair[1] - pair[0], 120, "Chill 啃食间隔 120：{drops:?}");
    }
}

#[test]
fn shield_ignored_by_spikes() {
    let mut sim = Sim::new(&[]);
    sim.defender("thornmat", 2, 5);
    let shield = sim.frozen_enemy("shield_husk", 2, col_x(5));
    sim.step(60);
    assert_eq!(sim.hp(shield), 9, "Spikes 直接扣主生命");
    assert_eq!(
        sim.world().get::<Armor>(shield).expect("护甲").hp,
        55,
        "Shield 不受影响"
    );
    let dmg = sim.events::<EnemyDamaged>();
    assert!(dmg.iter().all(|d| d.source == DamageSource::Spikes));
}

#[test]
fn mine_arms_after_900_ticks() {
    let mut sim = Sim::new(&[]);
    let mine = sim.defender("tuberblast", 2, 3);
    let clay = sim.frozen_enemy("clay_husk", 2, col_x(8));
    sim.step(898);
    sim.teleport(clay, col_x(3));
    sim.step(1);
    assert!(sim.alive(mine), "899 tick 时踩上不爆");
    assert_eq!(sim.hp(clay), 10);
    sim.step(1);
    assert!(!sim.world().contains(mine), "900 tick 武装后爆炸并消耗自身");
    let dmg = sim.events::<EnemyDamaged>();
    let hit = dmg.iter().find(|d| d.entity == clay).expect("受到伤害");
    assert_eq!((hit.amount, hit.source), (90, DamageSource::Mine));
    assert!(sim.events::<EnemyArmorBroken>().is_empty(), "无视护甲");
    assert!(died(&sim, clay));
}

#[test]
fn emberbulb_hits_3x3() {
    let mut sim = Sim::new(&[]);
    sim.defender("emberbulb", 2, 4);
    let inside = [
        sim.frozen_enemy("drum_husk", 1, col_x(3)),
        sim.frozen_enemy("drum_husk", 2, col_x(5)),
        sim.frozen_enemy("drum_husk", 3, col_x(4)),
    ];
    let outside = [
        sim.frozen_enemy("drum_husk", 0, col_x(4)),
        sim.frozen_enemy("drum_husk", 2, col_x(6)),
        sim.frozen_enemy("drum_husk", 4, col_x(4)),
    ];
    sim.step(60);
    let dmg = sim.events::<EnemyDamaged>();
    for e in inside {
        assert!(
            dmg.iter()
                .any(|d| d.entity == e && d.source == DamageSource::Blast && d.amount == 90),
            "3 × 3 内受伤"
        );
        assert!(
            sim.events::<EnemyArmorBroken>()
                .iter()
                .all(|b| b.entity != e),
            "无视护甲"
        );
    }
    for e in outside {
        assert!(dmg.iter().all(|d| d.entity != e), "外侧不受伤");
        assert_eq!(sim.hp(e), 30);
    }
}

#[test]
fn gourdshell_absorbs_first() {
    let mut sim = Sim::new(&[]);
    let wall = sim.defender("barkwall", 2, 3);
    let shell = sim.overlay("gourdshell", wall);
    sim.world_mut()
        .get_mut::<rootguard_core::components::Health>(shell)
        .expect("覆盖层")
        .hp = 3;
    sim.enemy("husk", 2, Grid::cell_right(3) + Grid::ENEMY_HALF_WIDTH);
    for _ in 0..180 {
        sim.step(1);
        assert_eq!(sim.hp(wall), 72, "覆盖层归零前宿主不掉血");
    }
    assert!(!sim.world().contains(shell), "第 3 口后覆盖层被移除");
    assert!(
        sim.world()
            .get::<rootguard_core::components::Shielded>(wall)
            .is_none()
    );
    sim.step(60);
    assert_eq!(sim.hp(wall), 71, "之后伤害落到宿主");
}

#[test]
fn snapmaw_swallows_and_chews() {
    let mut sim = Sim::new(&[]);
    let maw = sim.defender("snapmaw", 2, 2);
    let first = sim.frozen_enemy("husk", 2, col_x(3));
    sim.step(1);
    assert!(died(&sim, first), "进入右侧相邻格即被吞");
    let chomp = sim
        .events::<EnemyDamaged>()
        .into_iter()
        .find(|d| d.entity == first)
        .expect("吞食伤害");
    assert_eq!(chomp.source, DamageSource::Chomp);

    let heavy = sim.frozen_enemy("drum_husk", 2, col_x(3) + 30.0);
    let second = sim.frozen_enemy("husk", 2, col_x(3) + 40.0);
    sim.step(2519);
    assert!(sim.alive(second), "2520 tick 咀嚼期内不再吞");
    sim.step(1);
    assert!(sim.alive(second), "咀嚼结束的 tick 恢复就绪");
    sim.step(1);
    assert!(died(&sim, second), "恢复就绪后吞食非 Heavy 目标");
    assert!(sim.alive(heavy), "Heavy 不被吞");
    assert!(sim.alive(maw));
    sim.step(3000);
    assert!(sim.alive(heavy), "Heavy 始终不被吞");
}

#[test]
fn vault_husk_jumps_once() {
    let mut sim = Sim::new(&[]);
    let first_wall = sim.defender("barkwall", 2, 6);
    let second_wall = sim.defender("barkwall", 2, 3);
    let vaulter = sim.enemy("vault_husk", 2, col_x(8));
    let mut vaulted = false;
    let mut landed = false;
    for _ in 0..3000 {
        sim.step(1);
        let state = sim.world().get::<Walker>(vaulter).expect("Walker").state;
        match state {
            WalkState::Vaulting { .. } => vaulted = true,
            WalkState::Walking if vaulted && !landed => {
                landed = true;
                assert!((sim.x(vaulter) - col_x(5)).abs() < EPS, "落在左侧格中心");
            }
            WalkState::Eating(target) => {
                assert!(vaulted, "先跨越再啃食");
                assert_eq!(target, second_wall, "第二次遇墙正常啃食");
                assert_eq!(sim.hp(first_wall), 72, "第一面墙未被啃");
                return;
            }
            _ => {}
        }
    }
    panic!("3000 tick 内未发生第二次接触");
}

#[test]
fn carrier_spawns_sproutling() {
    let mut sim = Sim::new(&[]);
    sim.defender("pebble_pod", 4, 0);
    let carrier = sim.frozen_enemy("carrier_husk", 4, col_x(6));
    sim.world_mut()
        .get_mut::<rootguard_core::components::Health>(carrier)
        .expect("生命")
        .hp = 1;
    for _ in 0..1000 {
        sim.step(1);
        if died(&sim, carrier) {
            let death_x = sim
                .events::<EnemyDied>()
                .into_iter()
                .find(|d| d.entity == carrier)
                .expect("死亡事件")
                .x;
            let child = sim
                .events::<EnemySpawned>()
                .into_iter()
                .find(|s| s.id == eid("sproutling"))
                .expect("同 tick 生成 sproutling");
            assert_eq!(child.lane, 4);
            assert!((sim.x(child.entity) - death_x).abs() < EPS, "原地生成");
            assert!(!sim.world().contains(carrier));
            return;
        }
    }
    panic!("未击杀 carrier");
}

#[test]
fn sprinter_speeds_up_below_half() {
    let mut sim = Sim::new(&[]);
    let sprinter = sim.enemy("sprinter_husk", 0, col_x(8));
    let speed = |sim: &Sim| {
        sim.world()
            .get::<Walker>(sprinter)
            .expect("Walker")
            .base_speed
    };
    sim.step(1);
    assert!((speed(&sim) - 0.45).abs() < EPS);
    sim.world_mut()
        .get_mut::<rootguard_core::components::Health>(sprinter)
        .expect("生命")
        .hp = 6;
    sim.step(1);
    assert!((speed(&sim) - 0.45).abs() < EPS, "hp 6 时不触发");
    sim.world_mut()
        .get_mut::<rootguard_core::components::Health>(sprinter)
        .expect("生命")
        .hp = 5;
    sim.step(1);
    assert!((speed(&sim) - 0.675).abs() < EPS, "hp ≤ 5 后速度 0.675");
    let x0 = sim.x(sprinter);
    sim.step(4);
    assert!(((x0 - sim.x(sprinter)) - 4.0 * 0.675).abs() < POS_EPS);
}

#[test]
fn gust_pushes_and_clamps() {
    let mut sim = Sim::new(&[]);
    sim.defender("gustbloom", 2, 0);
    let light = sim.frozen_enemy("husk", 2, 500.0);
    let heavy = sim.frozen_enemy("drum_husk", 2, 600.0);
    let edge = sim.frozen_enemy("husk", 2, 1200.0);
    let other_lane = sim.frozen_enemy("husk", 1, 500.0);
    sim.step(59);
    assert!((sim.x(light) - 500.0).abs() < EPS, "引信期间不推");
    sim.step(1);
    assert!((sim.x(light) - 650.0).abs() < EPS, "推 150");
    assert!((sim.x(heavy) - 675.0).abs() < EPS, "Heavy 推 75");
    assert!((sim.x(edge) - 1260.0).abs() < EPS, "上限 1260");
    assert!((sim.x(other_lane) - 500.0).abs() < EPS, "只影响所在车道");
    let t = *sim.world().get::<Transform2D>(light).expect("Transform2D");
    assert_eq!(t.pos, t.prev_pos, "同步 prev_pos 避免插值拖影");
}

#[test]
fn hose_once_per_lane() {
    let mut sim = Sim::new(&[]);
    let first = sim.enemy("husk", 2, Grid::FAIL_X + 0.2);
    let bystander = sim.frozen_enemy("husk", 2, col_x(6));
    sim.step(1);
    assert_eq!(sim.events::<HoseTriggered>().len(), 1, "首次越线触发 Hose");
    assert!(died(&sim, first) && died(&sim, bystander), "清空该车道");
    assert_eq!(sim.state().phase, Phase::Playing);
    sim.step(1);
    sim.enemy("husk", 2, Grid::FAIL_X + 0.2);
    sim.step(1);
    assert_eq!(sim.state().phase, Phase::Lost, "第二次越线失败");
    let ended = sim.events::<BattleEnded>();
    assert_eq!(ended.last().map(|e| e.outcome), Some(Outcome::Lost));
}

fn husk_wave(delay: u32, count: u32, flag: bool) -> WaveDef {
    WaveDef {
        delay_ticks: delay,
        flag,
        spawns: vec![SpawnDef {
            enemy: eid("husk"),
            count,
            lane: LaneSpec::Fixed(1),
        }],
    }
}

#[test]
fn wave_early_start_at_40_percent() {
    let level = test_level(vec![husk_wave(10, 5, false), husk_wave(5000, 1, true)]);
    let mut sim = Sim::with_level(level, &[]);
    sim.step(10);
    assert_eq!(
        sim.events::<WaveStarted>().len(),
        1,
        "第一波在第 10 tick 生成"
    );
    let wave0: Vec<_> = sim
        .events::<EnemySpawned>()
        .iter()
        .map(|s| s.entity)
        .collect();
    assert_eq!(wave0.len(), 5);
    // 剩余 2/5 = 40% → 触发提前；最低间隔 300 tick
    sim.step(20);
    for &e in &wave0[..3] {
        sim.world_mut().despawn(e).expect("存在");
    }
    sim.step(1);
    let ws = *sim.world().resources.expect::<WaveState>();
    assert_eq!(ws.wave_hp_left, 20);
    let mut started = None;
    for _ in 0..400 {
        sim.step(1);
        if sim.events::<WaveStarted>().len() == 2 {
            started = Some(sim.state().level_tick);
            break;
        }
    }
    assert_eq!(
        started,
        Some(310),
        "上一波于 tick 10 生成，提前开始但间隔 ≥ 300"
    );

    // 间隔已满足时立即开始
    let level = test_level(vec![husk_wave(10, 5, false), husk_wave(5000, 1, true)]);
    let mut sim = Sim::with_level(level, &[]);
    sim.step(400);
    let wave0: Vec<_> = sim
        .events::<EnemySpawned>()
        .iter()
        .map(|s| s.entity)
        .collect();
    for &e in &wave0[..2] {
        sim.world_mut().despawn(e).expect("存在");
    }
    sim.step(1);
    assert_eq!(sim.events::<WaveStarted>().len(), 1, "剩余 60% 不提前");
    sim.world_mut().despawn(wave0[2]).expect("存在");
    sim.step(1);
    assert_eq!(sim.events::<WaveStarted>().len(), 2, "剩余 40% 立即开始");
}

#[test]
fn dew_cap_and_expire() {
    let mut sim = Sim::new(&[]);
    sim.world_mut().resources.expect_mut::<BattleState>().dew = 9980;
    let pos = Vec2::new(500.0, 300.0);
    let spawn_dew = |sim: &mut Sim, at: Vec2| {
        sim.world_mut().spawn((
            rootguard_core::components::BattleTag,
            Transform2D::new(at),
            DewPickup {
                amount: 25,
                land_y: at.y,
                expire: Timer::once(900),
                landed: true,
            },
        ))
    };
    let dew = spawn_dew(&mut sim, pos);
    sim.world_mut()
        .events
        .send(UiAction::CollectDewAt(pos + Vec2::new(20.0, 20.0)));
    sim.step(1);
    assert!(!sim.world().contains(dew), "点击碰撞圆内即收集");
    assert_eq!(sim.state().dew, 9990, "上限 9990");

    let dew = spawn_dew(&mut sim, pos);
    sim.step(899);
    assert!(sim.world().contains(dew));
    sim.step(1);
    assert!(!sim.world().contains(dew), "落地 900 tick 后消失");

    // 天空 Dew 从 y = -40 下落到目标格中心后才开始计时
    let level = rootguard_core::data::LevelDef {
        sky_dew: true,
        ..test_level(common::far_wave())
    };
    let mut sim = Sim::with_level(level, &[]);
    sim.step(420);
    let (e, t, d) = {
        let w = sim.world();
        let mut q = w.query::<(trellia_core::Entity, &Transform2D, &DewPickup)>();
        let (e, t, d) = q.iter().next().expect("第 420 tick 掉落天空 Dew");
        (e, *t, *d)
    };
    assert!((t.pos.y - (Grid::SKY_DEW_START_Y + 1.0)).abs() < EPS);
    assert!(!d.landed);
    let fall = (d.land_y - t.pos.y).ceil() as u32;
    sim.step(fall + 899);
    assert!(sim.world().contains(e), "落地前不计过期");
    sim.step(1);
    assert!(!sim.world().contains(e));
}

#[test]
fn card_ready_requires_cooldown_and_dew() {
    let mut sim = Sim::new(&["pebble_pod"]);
    let ready = |sim: &Sim| sim.world().resources.expect::<CardTray>().cards[0].ready;
    sim.world_mut().resources.expect_mut::<BattleState>().dew = 50;
    sim.step(1);
    assert!(!ready(&sim), "Dew 不足");
    sim.world_mut().resources.expect_mut::<BattleState>().dew = 100;
    sim.step(1);
    assert!(ready(&sim), "冷却完成且 Dew 足够");
    let events = &mut sim.world_mut().events;
    events.send(UiAction::SelectCard(0));
    events.send(UiAction::PlaceAt { row: 0, col: 0 });
    sim.step(1);
    assert_eq!(sim.state().dew, 0);
    sim.world_mut().resources.expect_mut::<BattleState>().dew = 9000;
    sim.step(1);
    assert!(!ready(&sim), "冷却中");
    sim.step(447);
    assert!(!ready(&sim));
    sim.step(1);
    assert!(ready(&sim), "450 tick 后冷却完成");
}

#[test]
fn placement_rules() {
    let level = rootguard_core::data::LevelDef {
        lanes: vec![1, 2, 3],
        ..test_level(common::far_wave())
    };
    let mut sim = Sim::with_level(level, &["pebble_pod", "gourdshell", "barkwall"]);
    sim.world_mut().resources.expect_mut::<BattleState>().dew = 1000;
    sim.step(1);
    let place = |sim: &mut Sim, card: usize, row: u8, col: u8| {
        let events = &mut sim.world_mut().events;
        events.send(UiAction::SelectCard(card));
        events.send(UiAction::PlaceAt { row, col });
        sim.step(1);
        sim.state().dew
    };
    assert_eq!(place(&mut sim, 0, 0, 0), 1000, "非激活车道拒绝且不扣费");
    assert_eq!(place(&mut sim, 1, 1, 0), 1000, "覆盖层不能放在空格");
    assert_eq!(place(&mut sim, 0, 1, 0), 900, "合法放置扣费");
    // 冷却中的卡不能选
    assert_eq!(place(&mut sim, 0, 1, 1), 900, "冷却中拒绝");
    assert_eq!(place(&mut sim, 2, 1, 0), 900, "已占格拒绝");
    assert_eq!(place(&mut sim, 1, 1, 0), 775, "覆盖层放在已有防御者上");
    sim.world_mut().resources.expect_mut::<CardTray>().cards[1]
        .cooldown
        .remaining = 0;
    assert_eq!(place(&mut sim, 1, 1, 0), 775, "满血覆盖层不能重复放置");
    sim.world_mut().resources.expect_mut::<BattleState>().dew = 20;
    assert_eq!(place(&mut sim, 2, 2, 0), 20, "Dew 不足拒绝");
}

#[test]
fn targeting_leftmost_then_entity_id() {
    let mut sim = Sim::new(&[]);
    sim.defender("pebble_pod", 2, 0);
    let near = sim.frozen_enemy("husk", 2, 700.0);
    let far = sim.frozen_enemy("husk", 2, 701.0);
    while sim.events::<ProjectileHit>().is_empty() {
        sim.step(1);
    }
    assert_eq!(sim.events::<ProjectileHit>()[0].target, near, "命中 x 小者");
    let _ = far;

    let mut sim = Sim::new(&[]);
    sim.defender("pebble_pod", 3, 0);
    let first = sim.frozen_enemy("husk", 3, 700.0);
    let second = sim.frozen_enemy("husk", 3, 700.0);
    assert!(first.to_bits() < second.to_bits());
    while sim.events::<ProjectileHit>().is_empty() {
        sim.step(1);
    }
    assert_eq!(
        sim.events::<ProjectileHit>()[0].target,
        first,
        "x 相同按 Entity id 升序"
    );
}

#[test]
fn damage_order_deterministic() {
    let run = |reverse_push: bool| {
        let mut sim = Sim::new(&[]);
        let a = sim.frozen_enemy("clay_husk", 1, col_x(8));
        let b = sim.frozen_enemy("clay_husk", 2, col_x(8));
        let mut reqs = vec![
            (a, 30, DamageSource::Projectile, false),
            (a, 5, DamageSource::Projectile, false),
            (b, 5, DamageSource::Projectile, false),
            (b, 30, DamageSource::Projectile, false),
        ];
        let queue = sim.world_mut().resources.expect_mut::<DamageQueue>();
        for &(t, amt, src, ign) in &reqs {
            queue.push(t, amt, src, ign);
        }
        if reverse_push {
            queue.requests.reverse();
            reqs.reverse();
        }
        damage_system(sim.world_mut());
        let log: Vec<(u64, i32)> = sim
            .events::<EnemyDamaged>()
            .iter()
            .map(|d| (d.entity.to_bits().get(), d.amount))
            .collect();
        (log, sim.hp(a), sim.hp(b))
    };
    let (log1, a1, b1) = run(false);
    let (log2, a2, b2) = run(true);
    assert_eq!(log1, log2, "结算顺序只由 (target, seq) 决定");
    assert_eq!((a1, b1), (a2, b2));
    // a：30 先打破 28 护甲并溢出 2，再 5 → 3；b：5 进护甲（剩 23），30 破甲溢出 7 → 3
    assert_eq!((a1, b1), (3, 3));
    let _ = lid;
}
