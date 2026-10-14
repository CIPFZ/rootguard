//! 皮肤模式（SPEC 12.1 / 12.3 / 17 G5）：皮肤覆盖全部内容、皮肤模式绘制精灵、两种模式切换对模拟无影响。

mod common;

use std::path::Path;

use common::{next_id, runner_with, send, state};
use rootguard_app::script::{Replay, Script};
use rootguard_app::skin::SkinAssets;
use rootguard_app::{BattleScene, build_app_with};
use rootguard_core::components::{BattleTag, Cell, DewPickup, EnemyRef, ProjectileBody};
use rootguard_core::data::EnemyDef;
use rootguard_core::events::{SkinMode, UiAction};
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
        .get::<SkinAssets>()
        .expect("皮肤应在启动时加载");
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
    let atlas = world
        .resources
        .expect::<Assets<Atlas>>()
        .get(skin.atlas())
        .expect("图集");
    for (id, _) in world.resources.expect::<EnemyRegistry>().iter() {
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
    // 音效才与伤害同频（负责人裁决 2026-10-15）。护甲破损后的 bare_eat 同样适用。
    let app = build_app_with(rootguard_app::TitleScene);
    let world = app.world();
    let skin = world.resources.expect::<SkinAssets>();
    let clips = world.resources.expect::<Assets<AnimationClip>>();
    let mut checked = 0;
    for (id, def) in world.resources.expect::<EnemyRegistry>().iter() {
        for name in [format!("{id}_eat"), format!("{id}_bare_eat")] {
            let Some(clip) = skin.clip(&name).and_then(|h| clips.get(h)) else {
                continue;
            };
            assert_eq!(
                rootguard_app::skin::clip_ticks(clip),
                u64::from(def.bite_interval_ticks),
                "{name} 总长应等于 {id} 的 bite_interval_ticks"
            );
            let hits = clip
                .frames
                .iter()
                .filter(|f| f.event.as_deref() == Some(rootguard_app::sfx::EAT_HIT_EVENT))
                .count();
            assert_eq!(hits, 1, "{name} 每循环应恰有 1 个 eat_hit 帧");
            checked += 1;
        }
    }
    assert!(checked >= 9, "应检查全部敌人的 eat 片段（{checked}）");
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
