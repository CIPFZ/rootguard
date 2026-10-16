//! 皮肤模式（SPEC 12.1 / 12.3 / 17 G5）：皮肤覆盖全部内容、皮肤模式绘制精灵、两种模式切换对模拟无影响。

mod common;

use std::path::Path;

use common::{next_id, runner_with, send, state};
use rootguard_app::script::{Replay, Script};
use rootguard_app::skin::Skins;
use rootguard_app::{BattleScene, build_app_with};
use rootguard_core::components::{
    BattleTag, Biter, Cell, DewPickup, EnemyRef, ProjectileBody, StatusEffects, WalkState, Walker,
};
use rootguard_core::data::{EnemyDef, Rules};
use rootguard_core::events::{SkinMode, SkinName, UiAction};
use rootguard_core::registry::{DefenderRegistry, EnemyRegistry, ProjectileRegistry};
use rootguard_core::resources::Phase;
use trellia::prelude::*;

fn script(id: &str) -> Script {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tests/scripts/{id}.ron"));
    Script::parse(&std::fs::read_to_string(path).expect("脚本")).expect("解析")
}

fn has_armor(def: &EnemyDef) -> bool {
    def.armor.is_some()
}

#[test]
fn skin_covers_all_content() {
    let app = build_app_with(rootguard_app::TitleScene);
    let world = app.world();
    let skin = world
        .resources
        .get::<Skins>()
        .and_then(|s| s.get(SkinName::Default))
        .expect("default 皮肤应在启动时加载");
    let mut missing = Vec::new();
    let mut need = |name: String| {
        if !skin.has_clip(&name) {
            missing.push(name);
        }
    };
    for (id, _) in world.resources.expect::<DefenderRegistry>().iter() {
        need(format!("{id}_idle"));
    }
    for (id, def) in world.resources.expect::<EnemyRegistry>().iter() {
        need(format!("{id}_walk"));
        need(format!("{id}_eat"));
        if has_armor(def) {
            need(format!("{id}_bare_walk"));
            need(format!("{id}_bare_eat"));
        }
    }
    for (id, _) in world.resources.expect::<ProjectileRegistry>().iter() {
        need(id.as_str().to_owned());
    }
    need(rootguard_app::render::DEW_CLIP.to_owned());
    assert!(missing.is_empty(), "缺少皮肤片段：{missing:?}");

    // SPEC 12.3：每个敌人的 eat 片段 3 帧，含 eat_hit 事件帧；每个敌人帧都有受伤闪白版
    let clips = world.resources.expect::<Assets<AnimationClip>>();
    for (id, _) in world.resources.expect::<EnemyRegistry>().iter() {
        let atlas = world
            .resources
            .expect::<Assets<Atlas>>()
            .get(skin.atlas_of(&format!("{id}_eat")).expect("eat 所在图集"))
            .expect("图集");
        let eat = clips
            .get(skin.clip(&format!("{id}_eat")).expect("eat"))
            .expect("eat 片段");
        assert_eq!(eat.frames.len(), 3, "{id}_eat 应为 3 帧");
        assert!(
            eat.frames
                .iter()
                .any(|f| f.event.as_deref() == Some("eat_hit")),
            "{id}_eat 缺少 eat_hit 事件帧"
        );
        let walk = clips
            .get(skin.clip(&format!("{id}_walk")).expect("walk"))
            .expect("walk 片段");
        assert_eq!(walk.frames.len(), 4, "{id}_walk 应为 4 帧");
        for f in walk.frames.iter().chain(&eat.frames) {
            let name = &atlas.frames[f.atlas_frame].name;
            assert!(
                atlas.frame(&format!("{name}_flash")).is_some(),
                "{name} 缺少闪白帧"
            );
        }
    }
    for (id, _) in world.resources.expect::<DefenderRegistry>().iter() {
        let idle = clips
            .get(skin.clip(&format!("{id}_idle")).expect("idle"))
            .expect("idle 片段");
        assert_eq!(idle.frames.len(), 4, "{id}_idle 应为 4 帧");
    }
}

#[test]
fn eat_clip_matches_bite_interval() {
    // eat 片段每循环进入一次 eat_hit 帧并播放 sfx.bite（ADR-010）：片段总长必须等于模拟中的啃食间隔，
    // 音效才与伤害同频（负责人裁决 2026-10-15）。护甲破损后的 bare_eat 同样适用；两套皮肤都检查。
    let app = build_app_with(rootguard_app::TitleScene);
    let world = app.world();
    let skins = world.resources.expect::<Skins>();
    let clips = world.resources.expect::<Assets<AnimationClip>>();
    let mut checked = 0;
    for name in SkinName::ALL {
        let skin = skins.get(name).expect("皮肤");
        for (id, def) in world.resources.expect::<EnemyRegistry>().iter() {
            for clip_name in [format!("{id}_eat"), format!("{id}_bare_eat")] {
                let Some(clip) = skin.clip(&clip_name).and_then(|h| clips.get(h)) else {
                    continue;
                };
                assert_eq!(
                    rootguard_app::skin::clip_ticks(clip),
                    u64::from(def.bite_interval_ticks),
                    "{}/{clip_name} 总长应等于 {id} 的 bite_interval_ticks",
                    name.dir()
                );
                let hits = clip
                    .frames
                    .iter()
                    .filter(|f| f.event.as_deref() == Some(rootguard_app::sfx::EAT_HIT_EVENT))
                    .count();
                assert_eq!(hits, 1, "{clip_name} 每循环应恰有 1 个 eat_hit 帧");
                checked += 1;
            }
        }
    }
    assert!(checked >= 10, "应检查全部敌人的 eat 片段（{checked}）");
}

/// 负责人裁决 2026-10-15（四）：敌人啃食时 eat 片段的相位由 `Biter` 计时器决定，而非游戏时钟。
/// 进度 p = elapsed / 有效间隔（Chill 时乘以规则倍率）映射到片段总长；`eat_hit` 帧是末帧，
/// 对应计时器到期前的最后一段。前半段正常，后半段让啃食者保持 Chill。
#[test]
fn eat_anim_phase_follows_biter_timer() {
    let mut runner = common::battle_runner("1-1", &[]);
    let mut checked = [0, 0];
    for _ in 0..4000 {
        let s = state(&runner).expect("对局");
        if s.level_tick == 60 {
            send(&mut runner, UiAction::SelectCard(0));
            send(&mut runner, UiAction::PlaceAt { row: 2, col: 8 });
        }
        let chill = checked[0] >= 150;
        if chill {
            for (w, st) in runner
                .app_mut()
                .world_mut()
                .entities
                .query_mut::<(&Walker, &mut StatusEffects)>()
            {
                if matches!(w.state, WalkState::Eating(_)) {
                    st.chill_ticks = 600;
                }
            }
        }
        runner.frame();
        let world = runner.app().world();
        let skin = world
            .resources
            .expect::<Skins>()
            .get(SkinName::Default)
            .expect("default 皮肤");
        let clips = world.resources.expect::<Assets<AnimationClip>>();
        let factor = world
            .resources
            .expect::<Rules>()
            .status
            .chill_bite_interval_factor;
        for (e, w, b, st) in world
            .query::<(Entity, &Walker, &Biter, &StatusEffects)>()
            .iter()
        {
            if !matches!(w.state, WalkState::Eating(_)) {
                continue;
            }
            let (name, elapsed) = rootguard_app::render::enemy_pose(world, e).expect("姿态");
            assert_eq!(name, "husk_eat");
            let clip = clips.get(skin.clip(&name).expect("片段")).expect("片段");
            let total = rootguard_app::skin::clip_ticks(clip);
            let interval = if st.chilled() {
                b.interval * factor
            } else {
                b.interval
            };
            assert_eq!(
                elapsed,
                u64::from(b.elapsed) * total / u64::from(interval),
                "eat 相位应为 Biter 进度 × 片段总长（chill = {}）",
                st.chilled()
            );
            let frame = rootguard_app::skin::frame_index_at(clip, elapsed);
            let last = clip.frames.len() - 1;
            assert_eq!(
                clip.frames[frame].event.as_deref() == Some("eat_hit"),
                frame == last,
                "eat_hit 应在末帧"
            );
            // 末帧 = 计时器剩余不超过末帧长度对应的 tick
            let last_ticks =
                u64::from(clip.frames[last].duration_ticks) * u64::from(interval) / total;
            assert_eq!(
                frame == last,
                u64::from(interval - b.elapsed) <= last_ticks,
                "末帧应对应计时器最后 {last_ticks} tick（elapsed {}）",
                b.elapsed
            );
            checked[usize::from(st.chilled())] += 1;
        }
        if checked[1] >= 300 {
            break;
        }
    }
    assert!(
        checked[0] >= 150 && checked[1] >= 300,
        "应在正常与 Chill 下都检查到啃食帧（{checked:?}）"
    );
}

/// 回放 1-4 脚本 `ticks` 帧；`skin_at(frame)` 给出该帧开始前要切换到的模式（`None` 不切换）。
fn run_1_4(
    ticks: u32,
    skin_at: impl Fn(u32) -> Option<SkinMode>,
    mut each: impl FnMut(&HeadlessRunner),
) -> HeadlessRunner {
    let script = script("1-4");
    let scene = BattleScene::new(script.level, script.cards.clone(), None);
    let mut runner = runner_with(scene, &format!("skin-{}", next_id()));
    let mut replay = Replay::new(script);
    for frame in 0..ticks {
        if let Some(mode) = skin_at(frame) {
            send(&mut runner, UiAction::SetSkin(mode));
        }
        if let Some(s) = state(&runner)
            && s.phase == Phase::Playing
        {
            for action in replay.due(s.level_tick) {
                send(&mut runner, action);
            }
        }
        runner.frame();
        each(&runner);
    }
    runner
}

#[test]
fn skinned_mode_draws_sprites() {
    let mut checked = 0;
    run_1_4(
        9000,
        |f| (f == 0).then_some(SkinMode::Skinned),
        |runner| {
            let world = runner.app().world();
            let actors = world.query::<(&BattleTag, &Cell)>().iter().count()
                + world.query::<(&BattleTag, &EnemyRef)>().iter().count()
                + world
                    .query::<(&BattleTag, &ProjectileBody)>()
                    .iter()
                    .count()
                + world.query::<(&BattleTag, &DewPickup)>().iter().count();
            let sprites = runner
                .last_draw_list()
                .iter()
                .filter(|c| matches!(c.kind, DrawKind::Sprite { .. }))
                .count();
            assert!(sprites >= actors, "精灵数 {sprites} < 角色数 {actors}");
            if actors > 5 {
                checked += 1;
            }
        },
    );
    assert!(checked > 1000, "应在有多个角色的帧上检查（{checked}）");
}

/// SPEC 17 G5“两种模式切换无差异”：对局中每 500 帧在两种模式间切换，30000 帧后的快照
/// 与只用占位模式生成的 `tests/golden/level_1_4.snap` 逐字节一致。
#[test]
fn skin_switching_keeps_golden_1_4() {
    let runner = run_1_4(
        30_000,
        |f| {
            f.is_multiple_of(500)
                .then_some(if (f / 500).is_multiple_of(2) {
                    SkinMode::Skinned
                } else {
                    SkinMode::Placeholder
                })
        },
        |_| {},
    );
    let mut bytes = runner.snapshot();
    bytes.extend(rootguard_core::snapshot::write(runner.app().world()));
    let golden = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/level_1_4.snap"),
    )
    .expect("golden");
    assert!(bytes == golden, "切换皮肤模式后快照与 golden 不一致");
}

/// garden 皮肤片段的 (所属角色 id, 动作)：`<id>[_<状态>]_<动作>`，id 取注册表中最长的前缀。
fn split_clip<'a>(name: &'a str, ids: &[String]) -> Option<(String, &'a str)> {
    let id = ids
        .iter()
        .filter(|id| name.starts_with(&format!("{id}_")))
        .max_by_key(|id| id.len())?;
    Some((id.clone(), name.rsplit('_').next()?))
}

/// ART-SPEC 3.1 / 9.2：garden 皮肤中由计时器驱动的片段时长等于规则数据，事件帧符合约定。
/// 读取的是提交的资产（游戏实际加载的内容），与 artgen 的构建期校验互为独立检查。
#[test]
fn art_clips_match_rules() {
    use rootguard_core::data::Behavior;
    let app = build_app_with(rootguard_app::TitleScene);
    let world = app.world();
    let garden = world
        .resources
        .expect::<Skins>()
        .get(SkinName::Garden)
        .expect("garden 皮肤");
    let clips = world.resources.expect::<Assets<AnimationClip>>();
    let defenders = world.resources.expect::<DefenderRegistry>();
    let enemies = world.resources.expect::<EnemyRegistry>();
    let ids: Vec<String> = defenders
        .iter()
        .map(|(id, _)| id.as_str().to_owned())
        .chain(enemies.iter().map(|(id, _)| id.as_str().to_owned()))
        .collect();
    let mut checked = Vec::new();
    for name in garden.clip_names() {
        let Some((id, act)) = split_clip(name, &ids) else {
            continue;
        };
        let clip = clips.get(garden.clip(name).expect("片段")).expect("片段");
        let total = rootguard_app::skin::clip_ticks(clip);
        let events = |e: &str| {
            clip.frames
                .iter()
                .filter(|f| f.event.as_deref() == Some(e))
                .count()
        };
        let behavior = defenders
            .iter()
            .find(|(d, _)| d.as_str() == id)
            .map(|(_, def)| def.behaviors.clone())
            .unwrap_or_default();
        let rule = match act {
            "eat" => enemies
                .iter()
                .find(|(e, _)| e.as_str() == id)
                .map(|(_, def)| def.bite_interval_ticks),
            "attack" => behavior.iter().find_map(|b| match b {
                Behavior::Shooter {
                    interval_ticks,
                    volley,
                    ..
                } => {
                    assert_eq!(
                        events("fire"),
                        usize::from(*volley),
                        "{name} 的 fire 数应等于连发数"
                    );
                    Some(*interval_ticks)
                }
                _ => None,
            }),
            "produce" => behavior.iter().find_map(|b| match b {
                Behavior::Producer { interval_ticks, .. } => Some(*interval_ticks),
                _ => None,
            }),
            "arm" => behavior.iter().find_map(|b| match b {
                Behavior::Mine { arm_ticks, .. } => Some(*arm_ticks),
                _ => None,
            }),
            "fuse" => behavior.iter().find_map(|b| match b {
                Behavior::Blast { fuse_ticks, .. } | Behavior::Gust { fuse_ticks, .. } => {
                    Some(*fuse_ticks)
                }
                _ => None,
            }),
            _ => continue,
        };
        let rule = rule.unwrap_or_else(|| panic!("{name}：{id} 没有与 {act} 对应的规则"));
        assert_eq!(
            total,
            u64::from(rule),
            "{name} 总长应等于规则时长 {rule}（ART-SPEC 3.1）"
        );
        if act == "eat" {
            assert_eq!(events("eat_hit"), 1, "{name} 应恰有一个 eat_hit");
            assert_eq!(
                clip.frames.last().and_then(|f| f.event.as_deref()),
                Some("eat_hit"),
                "{name} 的 eat_hit 应在末帧（ART-SPEC 3.1）"
            );
        }
        checked.push(name.to_owned());
    }
    for must in ["pebble_pod_attack", "husk_eat"] {
        assert!(
            checked.iter().any(|c| c == must),
            "应检查 {must}（{checked:?}）"
        );
    }
}

/// ART-SPEC 11 A0 / A2：garden 皮肤加载，切换到花园皮肤后石籽荚、枯壳怪、弹丸与 Dew 用 garden 图集绘制；
/// garden 中没有的片段回退 default 皮肤（ADR-014）。
#[test]
fn skin_garden_loads() {
    let mut runner = common::battle_runner("1-1", &[]);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    let garden_images = {
        let world = runner.app().world();
        let skins = world.resources.expect::<Skins>();
        let garden = skins.get(SkinName::Garden).expect("garden 皮肤应加载");
        for clip in [
            "pebble_pod_idle",
            "pebble_pod_attack",
            "pebble_pod_hurt_idle",
            "husk_walk",
            "husk_eat",
            "pebble_fly",
            "dew_idle",
        ] {
            assert!(garden.has_clip(clip), "garden 缺少 {clip}");
        }
        // 回退：garden 没有的片段从 default 取
        let view = skins.view(SkinName::Garden);
        let default = skins.get(SkinName::Default).expect("default");
        assert!(!garden.has_clip("dew") && default.has_clip("dew"));
        assert_eq!(
            view.entry("dew").map(|(a, _)| a),
            default.atlas_of("dew"),
            "garden 缺片段时应回退 default"
        );
        let atlases = world.resources.expect::<Assets<Atlas>>();
        let image = |clip: &str| {
            atlases
                .get(garden.atlas_of(clip).expect("图集"))
                .expect("图集")
                .image
        };
        [
            image("pebble_pod_idle"),
            image("husk_walk"),
            image("pebble_fly"),
            image("dew_idle"),
        ]
    };
    let mut seen = [false; 4];
    for _ in 0..3000 {
        let s = state(&runner).expect("对局");
        if s.level_tick == 60 {
            send(&mut runner, UiAction::SelectCard(0));
            send(&mut runner, UiAction::PlaceAt { row: 2, col: 3 });
        }
        runner.frame();
        for cmd in runner.last_draw_list().iter() {
            if let DrawKind::Sprite { image, .. } = &cmd.kind {
                for (i, g) in garden_images.iter().enumerate() {
                    seen[i] |= image == g;
                }
            }
        }
        if seen.iter().all(|s| *s) {
            break;
        }
    }
    assert_eq!(
        seen, [true; 4],
        "石籽荚、枯壳怪、弹丸、Dew 应以 garden 图集绘制"
    );
}

/// ART-SPEC 3.1 / ADR-014：石籽荚的 attack 片段相位由射手冷却计时器驱动——计时器刚重置（弹丸生成）的
/// 那一帧显示带 `fire` 事件的帧，之后逐 tick 对应；就绪且无目标时回到 idle。
#[test]
fn attack_anim_phase_follows_shooter_timer() {
    use rootguard_core::components::{DefenderRef, Shooter};
    let mut runner = common::battle_runner("1-1", &[]);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    let mut fired = 0;
    let mut idle = 0;
    for _ in 0..3000 {
        let s = state(&runner).expect("对局");
        if s.level_tick == 60 {
            send(&mut runner, UiAction::SelectCard(0));
            send(&mut runner, UiAction::PlaceAt { row: 2, col: 3 });
        }
        runner.frame();
        let world = runner.app().world();
        let clips = world.resources.expect::<Assets<AnimationClip>>();
        let garden = world
            .resources
            .expect::<Skins>()
            .get(SkinName::Garden)
            .expect("garden");
        for (e, shooter, _) in world.query::<(Entity, &Shooter, &DefenderRef)>().iter() {
            let (name, elapsed) = rootguard_app::render::defender_pose(world, e).expect("姿态");
            if shooter.timer.finished() {
                assert!(name.ends_with("_idle"), "就绪且无目标时应为 idle（{name}）");
                idle += 1;
                continue;
            }
            assert!(name.ends_with("_attack"), "冷却中应播放 attack（{name}）");
            let since = shooter.timer.duration - shooter.timer.remaining;
            assert_eq!(
                elapsed,
                u64::from(since),
                "attack 相位应等于计时器已过 tick"
            );
            let clip = clips.get(garden.clip(&name).expect("片段")).expect("片段");
            let frame = &clip.frames[rootguard_app::skin::frame_index_at(clip, elapsed)];
            if since == 0 {
                assert_eq!(
                    frame.event.as_deref(),
                    Some("fire"),
                    "弹丸生成的那一帧应是 fire 帧"
                );
                fired += 1;
            }
        }
    }
    assert!(
        fired >= 3 && idle > 0,
        "应观察到多次发射与就绪 idle（fired {fired}, idle {idle}）"
    );
}

/// 裁决（六）第 2 节：同一行的敌人重叠时按 x 从右到左绘制，最靠左者（弹丸先命中、正在闪白的那个，SPEC 2.5）
/// 画在最上面——否则整只白色闪白剪影被后画的同伴遮住，只露出一圈白边，看起来像错位的白影。
#[test]
fn overlapping_enemies_draw_leftmost_on_top() {
    use rootguard_core::components::{EnemyRef, Lane};
    let mut runner = common::battle_runner("1-1", &[]);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    let mut checked = 0;
    for _ in 0..9000 {
        let s = state(&runner).expect("对局");
        if s.level_tick == 60 {
            send(&mut runner, UiAction::SelectCard(0));
            send(&mut runner, UiAction::PlaceAt { row: 2, col: 3 });
        }
        runner.frame();
        let world = runner.app().world();
        let enemies = world.query::<(&EnemyRef, &Lane)>().iter().count();
        if enemies < 2 {
            continue;
        }
        let garden = world
            .resources
            .expect::<Skins>()
            .get(SkinName::Garden)
            .expect("garden");
        let husk_image = world
            .resources
            .expect::<Assets<Atlas>>()
            .get(garden.atlas_of("husk_walk").expect("图集"))
            .expect("图集")
            .image;
        let xs: Vec<f32> = runner
            .last_draw_list()
            .iter()
            .filter_map(|c| match &c.kind {
                DrawKind::Sprite { image, dst, .. } if *image == husk_image => Some(dst.min.x),
                _ => None,
            })
            .collect();
        assert!(
            xs.windows(2).all(|w| w[0] >= w[1]),
            "同一行的敌人应从右到左提交（{xs:?}）"
        );
        checked += 1;
    }
    assert!(checked > 100, "应在多只敌人同屏时检查（{checked}）");
}

/// ART-SPEC v1.2 8.1 / 裁决（七）第 4 节：花园皮肤画战场底图 `bg_base`（全屏），并按关卡给非激活车道叠加
/// 40% 遮罩（L3）；1-1 只有第 2 行激活，应有 4 条遮罩。
#[test]
fn garden_background_drawn_with_lane_mask() {
    let mut runner = common::battle_runner("1-1", &[]);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    runner.frames(2);
    let world = runner.app().world();
    let garden = world
        .resources
        .expect::<Skins>()
        .get(SkinName::Garden)
        .expect("garden");
    let bg_image = world
        .resources
        .expect::<Assets<Atlas>>()
        .get(
            garden
                .atlas_of(rootguard_app::render::BG_CLIP)
                .expect("bg_base"),
        )
        .expect("图集")
        .image;
    let list = runner.last_draw_list();
    let bg = list
        .iter()
        .filter_map(|c| match &c.kind {
            DrawKind::Sprite { image, dst, .. } if *image == bg_image => Some(*dst),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(bg.len(), 1, "应画一张底图");
    assert_eq!(bg[0].size(), Vec2::new(1280.0, 720.0));
    let masks = list
        .iter()
        .filter(|c| match &c.kind {
            DrawKind::Rect { rect, color, .. } => {
                (rect.size().x - 900.0).abs() < 0.5 && (color.a - 0.4).abs() < 0.01
            }
            _ => false,
        })
        .count();
    assert_eq!(masks, 4, "1-1 的 4 条非激活车道应各有一条遮罩");
}

/// ART-SPEC 3.1 / 4.6 / 4.10：咬夹草吞食后先播 snap（咀嚼计时器前 12 tick）再循环 chew、就绪时 idle；
/// 荆棘垫格内有敌人时播 trigger（相位 = 扎刺计时器），否则 idle。测试直接给足 Dew（只为放置）。
#[test]
fn chomper_and_spikes_clips_follow_state() {
    use rootguard_core::components::{Chomper, DefenderRef, Spikes};
    use rootguard_core::resources::BattleState;
    let mut runner = common::battle_runner("1-9", &["snapmaw", "thornmat"]);
    while state(&runner).is_some_and(|s| s.phase != Phase::Playing) {
        runner.frame();
    }
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..9000 {
        let s = state(&runner).expect("对局");
        {
            let world = runner.app_mut().world_mut();
            world.resources.expect_mut::<BattleState>().dew = 9990;
            for card in &mut world
                .resources
                .expect_mut::<rootguard_core::resources::CardTray>()
                .cards
            {
                card.cooldown.remaining = 0;
            }
        }
        // 每个 tick 放一张：第 30 ～ 39 tick 依次在 5 条车道放咬夹草（第 6 列）与荆棘垫（第 7 列）
        if (30..40).contains(&s.level_tick) {
            let k = s.level_tick - 30;
            let row = (k / 2) as u8;
            let (slot, col) = if k.is_multiple_of(2) { (0, 6) } else { (1, 7) };
            send(&mut runner, UiAction::SelectCard(slot));
            send(&mut runner, UiAction::PlaceAt { row, col });
        }
        runner.frame();
        let world = runner.app().world();
        for (e, c, _) in world.query::<(Entity, &Chomper, &DefenderRef)>().iter() {
            let (name, elapsed) = rootguard_app::render::defender_pose(world, e).expect("姿态");
            let since = u64::from(c.chew.duration - c.chew.remaining);
            let expect = match c.state {
                rootguard_core::components::ChompState::Chewing if since < 12 => "snapmaw_snap",
                rootguard_core::components::ChompState::Chewing => "snapmaw_chew",
                _ => "snapmaw_idle",
            };
            assert_eq!(name, expect);
            if expect == "snapmaw_snap" {
                assert_eq!(elapsed, since, "snap 相位应为吞食后经过的 tick");
            }
            seen.insert(expect);
        }
        for (e, sp, _) in world.query::<(Entity, &Spikes, &DefenderRef)>().iter() {
            let (name, elapsed) = rootguard_app::render::defender_pose(world, e).expect("姿态");
            if name == "thornmat_trigger" {
                assert_eq!(elapsed, u64::from(sp.timer.duration - sp.timer.remaining));
            } else {
                assert_eq!(name, "thornmat_idle");
            }
            seen.insert(if name == "thornmat_trigger" {
                "thornmat_trigger"
            } else {
                "thornmat_idle"
            });
        }
        if seen.len() == 5 {
            break;
        }
    }
    assert_eq!(
        seen.len(),
        5,
        "应观察到 snap / chew / idle 与 trigger / idle（{seen:?}）"
    );
}

/// ADR-016 第 2 节：生产者两层绘制——身体 idle（时钟）+ 叠层 produce（相位 = 生产计时器已过 tick，pop 在 0）；
/// 日灯笼的下一次生产是额外产量时叠层换成 produce_gold（ART-SPEC 4.12）。
#[test]
fn producer_overlay_follows_timer() {
    use rootguard_core::components::{DefenderRef, Producer};
    use rootguard_core::resources::BattleState;
    let mut runner = common::battle_runner("1-10", &["dewcup", "sunlantern"]);
    while state(&runner).is_some_and(|s| s.phase != Phase::Playing) {
        runner.frame();
    }
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    runner
        .app_mut()
        .world_mut()
        .resources
        .expect_mut::<BattleState>()
        .dew = 9990;
    send(&mut runner, UiAction::SelectCard(0));
    send(&mut runner, UiAction::PlaceAt { row: 0, col: 0 });
    runner.frame();
    send(&mut runner, UiAction::SelectCard(1));
    send(&mut runner, UiAction::PlaceAt { row: 1, col: 0 });
    let mut seen_gold = false;
    let mut checked = 0;
    for _ in 0..6000 {
        runner.frame();
        let world = runner.app().world();
        for (e, p, d) in world.query::<(Entity, &Producer, &DefenderRef)>().iter() {
            let (name, elapsed) = rootguard_app::render::producer_pose(world, e).expect("叠层");
            let (body, _) = rootguard_app::render::defender_pose(world, e).expect("身体");
            assert_eq!(body, format!("{}_idle", d.id.as_str()), "身体应播 idle");
            assert_eq!(elapsed, u64::from(p.timer.duration - p.timer.remaining));
            let bonus_next = p.bonus_every > 0 && (p.produced + 1) % p.bonus_every == 0;
            let expect = if bonus_next {
                "produce_gold"
            } else {
                "produce"
            };
            assert_eq!(name, format!("{}_{expect}", d.id.as_str()));
            seen_gold |= bonus_next;
            checked += 1;
        }
    }
    assert!(
        checked > 10_000 && seen_gold,
        "应检查两种生产者并见到金色一轮（{checked}）"
    );
}

/// ART-SPEC 3.1 / ADR-014：炽球与风铃花的 fuse 相位 = 引信计时器已过 tick；块茎雷武装前 arm 相位 = 武装计时器
/// 已过 tick，武装后循环 armed。
#[test]
fn fuse_and_arm_clips_follow_timers() {
    use rootguard_core::components::{Blast, DefenderRef, Gust, Mine};
    use rootguard_core::resources::BattleState;
    let mut runner = common::battle_runner("1-10", &["emberbulb", "gustbloom", "tuberblast"]);
    while state(&runner).is_some_and(|s| s.phase != Phase::Playing) {
        runner.frame();
    }
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    runner
        .app_mut()
        .world_mut()
        .resources
        .expect_mut::<BattleState>()
        .dew = 9990;
    // 卡牌就绪由上一 tick 的 Dew 计算，先推进一帧
    runner.frame();
    for (slot, col) in [(0u8, 1u8), (1, 2), (2, 3)] {
        send(&mut runner, UiAction::SelectCard(usize::from(slot)));
        send(&mut runner, UiAction::PlaceAt { row: 0, col });
        runner.frame();
    }
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..1000 {
        runner.frame();
        let world = runner.app().world();
        for (e, b, _) in world.query::<(Entity, &Blast, &DefenderRef)>().iter() {
            let (name, el) = rootguard_app::render::defender_pose(world, e).expect("姿态");
            assert_eq!(name, "emberbulb_fuse");
            assert_eq!(el, u64::from(b.fuse.duration - b.fuse.remaining).min(59));
            seen.insert("emberbulb_fuse");
        }
        for (e, g, _) in world.query::<(Entity, &Gust, &DefenderRef)>().iter() {
            let (name, el) = rootguard_app::render::defender_pose(world, e).expect("姿态");
            assert_eq!(name, "gustbloom_fuse");
            assert_eq!(el, u64::from(g.fuse.duration - g.fuse.remaining).min(59));
            seen.insert("gustbloom_fuse");
        }
        for (e, m, _) in world.query::<(Entity, &Mine, &DefenderRef)>().iter() {
            let (name, el) = rootguard_app::render::defender_pose(world, e).expect("姿态");
            if m.armed {
                assert_eq!(name, "tuberblast_armed");
                seen.insert("tuberblast_armed");
            } else {
                assert_eq!(name, "tuberblast_arm");
                assert_eq!(el, u64::from(m.arm.duration - m.arm.remaining));
                seen.insert("tuberblast_arm");
            }
        }
    }
    assert_eq!(seen.len(), 4, "应见到两种 fuse 与 arm / armed（{seen:?}）");
}

/// ADR-016 第 4 节：回放 1-10 脚本（全部敌人种类），花园皮肤下每个敌人每帧的片段与模拟状态一致——啃食 eat、
/// 跨越 vault、Sprint 触发后 sprint、跨越落地后 vaulted_、护甲 ≤ 1/2 时 cracked_、护甲没了 bare_、小芽怪生成后
/// spawn；行走相位 = 走过的距离 ÷ 步幅（`stride_elapsed`）。
#[test]
fn husk_clips_follow_movement_and_state() {
    use rootguard_core::components::{Armor, EnemyRef, Vaulter};
    use rootguard_core::registry::EnemyRegistry;
    use trellia::prelude::Transform2D as T2;
    let script = script("1-10");
    let scene = BattleScene::new(script.level, script.cards.clone(), None);
    let mut runner = runner_with(scene, &format!("husk-clips-{}", next_id()));
    let mut replay = Replay::new(script);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    let mut seen = std::collections::BTreeSet::new();
    runner.frame();
    for _ in 0..40_000 {
        let s = state(&runner).expect("对局");
        if matches!(s.phase, Phase::Won | Phase::Lost) {
            break;
        }
        if s.phase == Phase::Playing {
            for a in replay.due(s.level_tick) {
                send(&mut runner, a);
            }
        }
        runner.frame();
        let world = runner.app().world();
        let skin = world
            .resources
            .expect::<Skins>()
            .get(SkinName::Garden)
            .expect("garden");
        let clips = world.resources.expect::<Assets<AnimationClip>>();
        let reg = world.resources.expect::<EnemyRegistry>();
        for (e, enemy, t, w, armor, vaulter) in world
            .query::<(
                Entity,
                &EnemyRef,
                &T2,
                &Walker,
                Option<&Armor>,
                Option<&Vaulter>,
            )>()
            .iter()
        {
            let id = enemy.id.as_str();
            let (name, elapsed) = rootguard_app::render::enemy_pose(world, e).expect("姿态");
            let vaulting = matches!(w.state, WalkState::Vaulting { .. });
            if vaulter.is_some_and(|v| v.used) && !vaulting {
                assert!(name.contains("_vaulted_"), "{name}");
                seen.insert("vaulted");
            } else if let Some(a) = armor.filter(|a| a.hp > 0) {
                if a.hp * 2 <= a.max {
                    assert!(name.contains("_cracked_"), "{name}");
                    seen.insert("cracked");
                } else {
                    assert!(
                        !name.contains("_cracked_") && !name.contains("_bare_"),
                        "{name}"
                    );
                }
            } else if reg.get(enemy.id).is_some_and(|d| d.armor.is_some()) {
                assert!(name.contains("_bare_"), "{name}");
                seen.insert("bare");
            }
            match w.state {
                WalkState::Eating(_) => {
                    assert!(name.ends_with("_eat"), "{name}");
                    seen.insert("eat");
                }
                WalkState::Vaulting { .. } => {
                    assert!(name.ends_with("_vault"), "{name}");
                    seen.insert("vault");
                }
                _ if name.ends_with("_spawn") => {
                    seen.insert("spawn");
                }
                _ => {
                    let speed = reg.get(enemy.id).map_or(0.0, |d| d.speed);
                    let sprinting = w.base_speed > speed + f32::EPSILON;
                    if sprinting && skin.has_clip(&format!("{id}_sprint")) {
                        assert!(name.ends_with("_sprint"), "{name}");
                        seen.insert("sprint");
                    } else {
                        assert!(name.ends_with("_walk"), "{name}");
                    }
                    let len = rootguard_app::skin::clip_ticks(
                        clips.get(skin.clip(&name).expect("片段")).expect("片段"),
                    );
                    assert_eq!(
                        elapsed,
                        rootguard_app::render::stride_elapsed(t.pos.x, w.base_speed, len),
                        "{name} 的相位应由走过的距离决定"
                    );
                    seen.insert("walk");
                }
            }
        }
        if seen.len() >= 8 {
            break;
        }
    }
    for k in [
        "walk", "eat", "cracked", "bare", "sprint", "vault", "vaulted", "spawn",
    ] {
        assert!(seen.contains(k), "1-10 回放中应见到 {k}（{seen:?}）");
    }
}

/// ART-SPEC 7 / ADR-016 第 5 节：回放 1-10，敌人护甲破碎时生成对应的 `armor_break_*` 特效（同一事件只生成一次），
/// 特效按片段长度到期删除。
#[test]
fn armor_break_spawns_effects() {
    use rootguard_core::components::EnemyRef;
    use rootguard_core::data::ArmorKind;
    use rootguard_core::events::EnemyArmorBroken;
    let mut seen = std::collections::BTreeSet::new();
    let script = script("1-10");
    let scene = BattleScene::new(script.level, script.cards.clone(), None);
    let mut runner = runner_with(scene, &format!("armor-fx-{}", next_id()));
    let mut replay = Replay::new(script);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    runner.frame();
    let mut injected = false;
    for _ in 0..40_000 {
        let s = state(&runner).expect("对局");
        if matches!(s.phase, Phase::Won | Phase::Lost) {
            break;
        }
        if s.phase == Phase::Playing {
            for a in replay.due(s.level_tick) {
                send(&mut runner, a);
            }
        }
        // 全部脚本解里门板都死于无视 Shield 的伤害、从未破碎：对第一只门板枯壳注入一次破碎事件（只有呈现层读取）
        if !injected {
            let shield = runner
                .app()
                .world()
                .query::<(Entity, &EnemyRef)>()
                .iter()
                .find(|(_, r)| r.id.as_str() == "shield_husk")
                .map(|(e, _)| e);
            if let Some(entity) = shield {
                runner.app_mut().world_mut().events.send(EnemyArmorBroken {
                    entity,
                    kind: ArmorKind::Shield,
                });
                injected = true;
            }
        }
        runner.frame();
        let world = runner.app().world();
        let tick = world.resources.expect::<Time>().fixed_tick;
        let rs = world
            .resources
            .expect::<rootguard_app::render::RenderState>();
        let mut starts = std::collections::BTreeSet::new();
        for f in rs.effects() {
            if f.clip.starts_with("armor_break_") {
                assert!(tick - f.start < 16, "{} 应在 16 tick 内删除", f.clip);
                // 同一位置同一 tick 不应出现两份（双缓冲事件去重）
                let k = (
                    f.clip.clone(),
                    f.start,
                    f.pos.x.to_bits(),
                    f.pos.y.to_bits(),
                );
                assert!(starts.insert(k), "{} 重复生成", f.clip);
                seen.insert(f.clip.clone());
            }
        }
    }
    for fx in ["twig", "pot", "plank", "gourd"] {
        let clip = format!("armor_break_{fx}_play");
        assert!(seen.contains(&clip), "应见到 {clip}（{seen:?}）");
    }
}

/// ART-SPEC 2.4：树皮墙与葫芦壳 3 级受损（> 2/3 完好、> 1/3 cracked、其余 split）；石籽荚 2 级（≤ 1/2 hurt）。
/// 测试直接改写生命值只为构造状态。
#[test]
fn defender_damage_states() {
    use rootguard_core::components::{DefenderRef, Health};
    use rootguard_core::resources::BattleState;
    let mut runner = common::battle_runner("1-10", &["barkwall", "pebble_pod", "gourdshell"]);
    while state(&runner).is_some_and(|s| s.phase != Phase::Playing) {
        runner.frame();
    }
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    runner
        .app_mut()
        .world_mut()
        .resources
        .expect_mut::<BattleState>()
        .dew = 9990;
    runner.frame();
    for (slot, col) in [(0usize, 0u8), (1, 1), (2, 1)] {
        send(&mut runner, UiAction::SelectCard(slot));
        send(&mut runner, UiAction::PlaceAt { row: 0, col });
        runner.frame();
    }
    let ids: Vec<(Entity, String)> = runner
        .app()
        .world()
        .query::<(Entity, &DefenderRef)>()
        .iter()
        .map(|(e, d)| (e, d.id.as_str().to_owned()))
        .collect();
    assert_eq!(ids.len(), 3);
    for (frac, wall, pod) in [
        (1.0, "", ""),
        (0.6, "_cracked", ""),
        (0.5, "_cracked", "_hurt"),
        (0.3, "_split", "_hurt"),
    ] {
        for (e, _) in &ids {
            let world = runner.app_mut().world_mut();
            let mut h = world.get_mut::<Health>(*e).expect("生命");
            h.hp = ((h.max as f32) * frac).round() as i32;
        }
        let world = runner.app().world();
        for (e, id) in &ids {
            let (name, _) = rootguard_app::render::defender_pose(world, *e).expect("姿态");
            let expect = match id.as_str() {
                "pebble_pod" => format!("pebble_pod{pod}_idle"),
                other => format!("{other}{wall}_idle"),
            };
            assert_eq!(name, expect, "hp 比例 {frac}");
        }
    }
}

/// ART-SPEC 7 / ADR-016 第 6 节：回放 1-10，各类事件生成对应特效；Chill 中的 Husk 头顶画霜晶（3.3）。
#[test]
fn battle_effects_follow_events() {
    use rootguard_core::components::{EnemyRef, StatusEffects as St};
    let script = script("1-10");
    let cards: Vec<String> = script.cards.iter().map(|c| c.as_str().to_owned()).collect();
    let scene = BattleScene::new(script.level, script.cards.clone(), None);
    let mut runner = runner_with(scene, &format!("fx-{}", next_id()));
    let mut replay = Replay::new(script);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    runner.frame();
    let mut seen = std::collections::BTreeSet::new();
    let mut crown_checked = 0;
    for _ in 0..40_000 {
        let s = state(&runner).expect("对局");
        if matches!(s.phase, Phase::Won | Phase::Lost) {
            break;
        }
        if s.phase == Phase::Playing {
            for a in replay.due(s.level_tick) {
                send(&mut runner, a);
            }
        }
        runner.frame();
        let world = runner.app().world();
        let rs = world
            .resources
            .expect::<rootguard_app::render::RenderState>();
        for f in rs.effects() {
            seen.insert(f.clip.trim_end_matches("_play").to_owned());
        }
        let chilled = world
            .query::<(&EnemyRef, &St)>()
            .iter()
            .filter(|(_, st)| st.chilled())
            .count();
        if chilled > 0 {
            let garden = world
                .resources
                .expect::<Skins>()
                .get(SkinName::Garden)
                .expect("garden");
            let crown = world
                .resources
                .expect::<Assets<Atlas>>()
                .get(garden.atlas_of("frost_crown_play").expect("霜晶"))
                .expect("图集")
                .frames
                .iter()
                .filter(|f| f.name.starts_with("frost_crown_play_"))
                .map(|f| f.src_px)
                .collect::<Vec<_>>();
            let drawn = runner
                .last_draw_list()
                .iter()
                .filter(|c| matches!(&c.kind, DrawKind::Sprite { src_px, .. } if crown.contains(src_px)))
                .count();
            assert!(
                drawn >= chilled,
                "每个 Chill 中的 Husk 应画霜晶（{drawn} < {chilled}）"
            );
            crown_checked += 1;
        }
    }
    let _ = cards;
    // 1-10 脚本解没有用到双籽荚（石子）与风铃花，这两类在 constructed_effects 中检查
    for fx in [
        "muzzle_frost",
        "hit_frost",
        "crumble",
        "sprout_fold",
        "plant_dust",
        "dew_pop",
        "wilt",
        "tuber_burst",
        "ember_wave",
        "hose_burst",
    ] {
        assert!(seen.contains(fx), "1-10 中应见到 {fx}（{seen:?}）");
    }
    assert!(
        crown_checked > 100,
        "应在有 Chill 敌人的帧上检查霜晶（{crown_checked}）"
    );
}

/// ADR-016 第 6 节：脚本解里没有出现的特效——石籽荚的口部小尘与石子命中、风铃花风环（沿车道拉伸到右端）、
/// 铲除残影（4.13，12 tick 上升淡出）、放置的弹性缩放。测试给足 Dew 只为放置。
#[test]
fn constructed_effects() {
    use rootguard_core::resources::BattleState;
    let mut runner = common::battle_runner("1-10", &["pebble_pod", "gustbloom", "dewcup"]);
    while state(&runner).is_some_and(|s| s.phase != Phase::Playing) {
        runner.frame();
    }
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    let refill = |runner: &mut HeadlessRunner| {
        let world = runner.app_mut().world_mut();
        world.resources.expect_mut::<BattleState>().dew = 9990;
        for card in &mut world
            .resources
            .expect_mut::<rootguard_core::resources::CardTray>()
            .cards
        {
            card.cooldown.remaining = 0;
        }
    };
    refill(&mut runner);
    runner.frame();
    for row in 0..5 {
        refill(&mut runner);
        send(&mut runner, UiAction::SelectCard(0));
        send(&mut runner, UiAction::PlaceAt { row, col: 0 });
        runner.frame();
    }
    refill(&mut runner);
    send(&mut runner, UiAction::SelectCard(2));
    send(&mut runner, UiAction::PlaceAt { row: 0, col: 4 });
    runner.frame();
    let mut seen = std::collections::BTreeMap::new();
    let mut gust_placed = false;
    let mut shoveled = false;
    for _ in 0..6000 {
        let s = state(&runner).expect("对局");
        refill(&mut runner);
        let enemies = runner.app().world().query::<&EnemyRef>().iter().count();
        if !gust_placed && enemies >= 2 {
            send(&mut runner, UiAction::SelectCard(1));
            send(&mut runner, UiAction::PlaceAt { row: 2, col: 2 });
            gust_placed = true;
        }
        if !shoveled && s.level_tick > 600 {
            send(&mut runner, UiAction::ToggleShovel);
            send(&mut runner, UiAction::ShovelAt { row: 0, col: 4 });
            shoveled = true;
        }
        runner.frame();
        let world = runner.app().world();
        let rs = world
            .resources
            .expect::<rootguard_app::render::RenderState>();
        for f in rs.effects() {
            seen.entry(f.clip.clone())
                .or_insert((f.size, f.pos, f.ghost));
        }
        if [
            "muzzle_dust_play",
            "hit_puff_play",
            "gust_ring_play",
            "dewcup_idle",
        ]
        .iter()
        .all(|k| seen.contains_key(*k))
        {
            break;
        }
    }
    for k in [
        "muzzle_dust_play",
        "hit_puff_play",
        "gust_ring_play",
        "dewcup_idle",
    ] {
        assert!(
            seen.contains_key(k),
            "应见到 {k}（{:?}）",
            seen.keys().collect::<Vec<_>>()
        );
    }
    let (size, pos, _) = seen["gust_ring_play"];
    let size = size.expect("风环拉伸");
    assert!(
        (pos.x + size.x / 2.0 - rootguard_core::Grid::GUST_CLAMP_X).abs() < 0.5,
        "风环应扫到车道右端"
    );
    assert!(seen["dewcup_idle"].2, "铲除应生成残影");
    assert!((rootguard_app::render::plant_scale(0) - 1.2).abs() < 1e-3);
    assert!((rootguard_app::render::plant_scale(10) - 1.0).abs() < 1e-6);
}

/// 绘制列表中某片段第 0 帧的全部目标矩形（只按原图区域的位置比较，裁剪过的也算）。
fn drawn_rects(runner: &HeadlessRunner, clip: &str) -> Vec<Rect> {
    drawn_rects_opt(runner, clip).unwrap_or_default()
}

/// 同 [`drawn_rects`]；一次都没画时为 `None`。
fn drawn_rects_opt(runner: &HeadlessRunner, clip: &str) -> Option<Vec<Rect>> {
    let world = runner.app().world();
    let garden = world
        .resources
        .expect::<Skins>()
        .get(SkinName::Garden)
        .expect("garden");
    let atlases = world.resources.expect::<Assets<Atlas>>();
    let atlas = atlases
        .get(garden.atlas_of(clip).expect("片段所在图集"))
        .expect("图集");
    let src = atlas
        .frame(&format!("{clip}_0"))
        .map(|i| atlas.frames[i].src_px)
        .expect("帧");
    let rects: Vec<Rect> = runner
        .last_draw_list()
        .iter()
        .filter_map(|c| match &c.kind {
            DrawKind::Sprite {
                image, src_px, dst, ..
            } if *image == atlas.image && src_px.min == src.min && src_px.max.y == src.max.y => {
                Some(*dst)
            }
            _ => None,
        })
        .collect();
    (!rects.is_empty()).then_some(rects)
}

/// ART-SPEC 8.1 L4 ～ L6：花园皮肤画温室外墙、入口、前景与每行的门柱、阀门；某行的 Hose 用掉后该行阀门
/// 换成 `spent`（测试直接改写 `LaneState` 只为构造状态）。
#[test]
fn garden_scene_valves_follow_hose() {
    use rootguard_app::render::{
        SCENE_GATE_POST, SCENE_VALVE_CLOSED, SCENE_VALVE_SPENT, SCENE_WALL,
    };
    use rootguard_core::resources::LaneState;
    let mut runner = common::battle_runner("1-10", &[]);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    runner.frames(2);
    assert_eq!(drawn_rects(&runner, SCENE_WALL).len(), 1);
    assert_eq!(drawn_rects(&runner, SCENE_GATE_POST).len(), 5);
    assert_eq!(drawn_rects(&runner, SCENE_VALVE_CLOSED).len(), 5);
    assert!(drawn_rects(&runner, SCENE_VALVE_SPENT).is_empty());
    runner
        .app_mut()
        .world_mut()
        .resources
        .expect_mut::<LaneState>()
        .hose_available[2] = false;
    runner.frame();
    let spent = drawn_rects(&runner, SCENE_VALVE_SPENT);
    assert_eq!(spent.len(), 1);
    assert!(
        (spent[0].center().y - rootguard_core::Grid::row_center_y(2)).abs() < 20.0,
        "spent 阀门应在第 2 行"
    );
    assert_eq!(drawn_rects(&runner, SCENE_VALVE_CLOSED).len(), 4);
}

/// ART-SPEC 8.3 / 4.13：花园皮肤的 HUD 画木条、卡槽木牌与卡牌头像（不可用时去饱和版）、园丁小铁铲、藤蔓波次条
/// （已到波次按进度裁出左段）；默认皮肤仍是占位 HUD。
#[test]
fn garden_hud_sprites() {
    use rootguard_app::hud::{UI_HUD_BAR, UI_SHOVEL, UI_WAVE_FILL};
    use rootguard_core::resources::WaveState;
    let mut runner = common::battle_runner("1-10", &[]);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    runner.frames(2);
    assert!(
        drawn_rects_opt(&runner, UI_HUD_BAR).is_none(),
        "default 皮肤没有 HUD 精灵"
    );
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    runner.frames(2);
    assert_eq!(drawn_rects(&runner, UI_HUD_BAR).len(), 1);
    assert_eq!(drawn_rects(&runner, UI_SHOVEL).len(), 1);
    // 头像：可用（ready 且不在冷却）为原图，否则为去饱和版；至少两种都出现
    let cards = runner
        .app()
        .world()
        .resources
        .expect::<rootguard_core::resources::CardTray>()
        .cards
        .clone();
    let mut kinds = std::collections::BTreeSet::new();
    for c in &cards {
        let id = c.defender.as_str();
        let usable = c.ready && c.cooldown.remaining == 0;
        let (want, other) = if usable {
            (format!("{id}_portrait"), format!("{id}_portrait_off"))
        } else {
            (format!("{id}_portrait_off"), format!("{id}_portrait"))
        };
        assert_eq!(drawn_rects(&runner, &want).len(), 1, "{want}");
        assert!(drawn_rects(&runner, &other).is_empty(), "{other}");
        kinds.insert(usable);
    }
    assert_eq!(kinds.len(), 2, "应同时有可用与不可用的卡");
    // 波次藤：进度 = 已生成波 / 总波
    {
        let world = runner.app_mut().world_mut();
        let w = world.resources.expect_mut::<WaveState>();
        w.next_wave = w.total_waves / 2;
    }
    runner.frame();
    let waves = runner.app().world().resources.expect::<WaveState>();
    let k = waves.next_wave as f32 / waves.total_waves as f32;
    let fill = drawn_rects(&runner, UI_WAVE_FILL);
    assert_eq!(fill.len(), 1);
    assert!(
        (fill[0].size().x - 210.0 * k).abs() < 0.5,
        "藤的长度应为进度 × 210"
    );
}
