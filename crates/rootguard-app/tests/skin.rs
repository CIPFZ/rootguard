//! 皮肤模式（SPEC 12.1 / 12.3 / 17 G5）：皮肤覆盖全部内容、皮肤模式绘制精灵、两种模式切换对模拟无影响。

mod common;

use std::path::Path;

use common::{next_id, runner_with, send, state};
use rootguard_app::script::{Replay, Script};
use rootguard_app::skin::{SkinAssets, Skins};
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

/// ART-SPEC 11 A0：garden 皮肤加载，切换到花园皮肤后石籽荚与枯壳怪用 garden 图集绘制；garden 中没有的
/// 角色回退 default 皮肤（ADR-014）。
#[test]
fn skin_garden_loads() {
    let mut runner = common::battle_runner("1-1", &[]);
    send(&mut runner, UiAction::SetSkin(SkinMode::Skinned));
    send(&mut runner, UiAction::SetSkinName(SkinName::Garden));
    let (garden_images, default_image) = {
        let world = runner.app().world();
        let skins = world.resources.expect::<Skins>();
        let garden = skins.get(SkinName::Garden).expect("garden 皮肤应加载");
        for clip in [
            "pebble_pod_idle",
            "pebble_pod_attack",
            "pebble_pod_hurt_idle",
            "husk_walk",
            "husk_eat",
        ] {
            assert!(garden.has_clip(clip), "garden 缺少 {clip}");
        }
        let atlases = world.resources.expect::<Assets<Atlas>>();
        let image = |skin: &SkinAssets, clip: &str| {
            atlases
                .get(skin.atlas_of(clip).expect("图集"))
                .expect("图集")
                .image
        };
        let default = skins.get(SkinName::Default).expect("default");
        (
            [image(garden, "pebble_pod_idle"), image(garden, "husk_walk")],
            image(default, "dew"),
        )
    };
    let mut seen_garden = [false, false];
    let mut seen_fallback = false;
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
                    seen_garden[i] |= image == g;
                }
                seen_fallback |= *image == default_image;
            }
        }
        if seen_garden == [true, true] && seen_fallback {
            break;
        }
    }
    assert_eq!(
        seen_garden,
        [true, true],
        "石籽荚与枯壳怪应以 garden 图集绘制"
    );
    assert!(seen_fallback, "garden 中没有的 Dew 应回退 default 皮肤绘制");
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
