//! SPEC 16.2 的 G3 验收：UI 流程、存档往返、音频事件映射；以及文本表完整性（SPEC 11.6）。

mod common;

use std::path::Path;

use common::{battle_runner, lid, next_id, send, state, temp_save, top};
use rootguard_app::i18n::I18n;
use rootguard_app::save::{self, BACKUP_NAME, FILE_NAME, SaveLocation, SaveSlot};
use rootguard_app::script::{Replay, Script};
use rootguard_app::sfx::AudioBank;
use rootguard_app::{TitleScene, build_app_full};
use rootguard_core::components::{DefenderRef, Health, StatusEffects, WalkState, Walker};
use rootguard_core::data::{CardRule, Rules};
use rootguard_core::events::{Lang, UiAction, VolumeBus};
use rootguard_core::registry::{DefenderRegistry, EnemyRegistry, LevelRegistry};
use rootguard_core::resources::{Phase, Progress};
use trellia::prelude::*;

fn title_runner(dir: &Path) -> HeadlessRunner {
    let mut runner = HeadlessRunner::new(build_app_full(
        TitleScene,
        SaveLocation::Dir(dir.to_path_buf()),
    ));
    runner.frame();
    runner
}

fn act(runner: &mut HeadlessRunner, action: UiAction) {
    send(runner, action);
    runner.frames(2);
}

/// 回放某关脚本直到结束，返回结束阶段。
fn play_script(runner: &mut HeadlessRunner, id: &str) -> Option<Phase> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tests/scripts/{id}.ron"));
    let script = Script::parse(&std::fs::read_to_string(path).expect("脚本")).expect("解析");
    let mut replay = Replay::new(script);
    for _ in 0..100_000 {
        let s = state(runner)?;
        if matches!(s.phase, Phase::Won | Phase::Lost) {
            runner.frames(2);
            return Some(s.phase);
        }
        if s.phase == Phase::Playing {
            for a in replay.due(s.level_tick) {
                send(runner, a);
            }
        }
        runner.frame();
    }
    None
}

#[test]
fn ui_flow_title_to_battle() {
    let dir = temp_save(&format!("flow-{}", next_id()));
    let mut runner = title_runner(&dir);
    assert_eq!(top(&runner), Some("Title"));
    act(&mut runner, UiAction::BackToLevelSelect);
    assert_eq!(top(&runner), Some("LevelSelect"));

    // 第一关（Forced）直接进入对局
    act(&mut runner, UiAction::StartLevel(lid("1-1")));
    assert_eq!(top(&runner), Some("Battle"));
    act(&mut runner, UiAction::PauseToggle);
    assert_eq!(top(&runner), Some("Paused"));
    let frozen = state(&runner).expect("对局").level_tick;
    runner.frames(30);
    assert_eq!(
        state(&runner).expect("对局").level_tick,
        frozen,
        "暂停时模拟不推进"
    );
    act(&mut runner, UiAction::PauseToggle);
    assert_eq!(top(&runner), Some("Battle"));

    // 未解锁的关卡拒绝进入
    act(&mut runner, UiAction::BackToLevelSelect);
    let levels: Vec<_> = runner
        .app()
        .world()
        .resources
        .expect::<LevelRegistry>()
        .iter()
        .cloned()
        .collect();
    if let Some(second) = levels.get(1) {
        act(&mut runner, UiAction::StartLevel(second.id));
        assert_eq!(top(&runner), Some("LevelSelect"), "上一关未通关时不能进入");
    }

    // Choose 关：StartLevel → SeedSelect → ConfirmSeeds → Battle
    if let Some(choose) = levels.iter().position(|l| l.cards == CardRule::Choose) {
        let world = runner.app_mut().world_mut();
        let progress = world.resources.expect_mut::<Progress>();
        for l in &levels[..choose] {
            progress.cleared.insert(l.id);
        }
        act(&mut runner, UiAction::StartLevel(levels[choose].id));
        assert_eq!(top(&runner), Some("SeedSelect"));
        let cards: Vec<_> = runner
            .app()
            .world()
            .resources
            .expect::<Progress>()
            .unlocked
            .iter()
            .copied()
            .collect();
        act(&mut runner, UiAction::ConfirmSeeds(cards));
        assert_eq!(top(&runner), Some("Battle"));
    }
    act(&mut runner, UiAction::BackToTitle);
    assert_eq!(top(&runner), Some("Title"));
}

#[test]
fn save_roundtrip() {
    let dir = temp_save(&format!("save-{}", next_id()));
    let mut runner = title_runner(&dir);
    assert!(!runner.app().world().resources.expect::<SaveSlot>().existed);
    act(
        &mut runner,
        UiAction::SetVolume {
            bus: VolumeBus::Music,
            volume: 0.25,
        },
    );
    act(&mut runner, UiAction::SetLanguage(Lang::En));
    act(&mut runner, UiAction::StartLevel(lid("1-1")));
    assert_eq!(play_script(&mut runner, "1-1"), Some(Phase::Won));
    assert_eq!(top(&runner), Some("Victory"));
    let saved = runner.app().world().resources.expect::<Progress>().clone();
    assert!(saved.cleared.contains(&lid("1-1")));
    assert!(
        saved.unlocked.iter().any(|d| d.as_str() == "dewcup"),
        "通关解锁 dewcup"
    );
    assert!((saved.settings.music - 0.25).abs() < 1e-6);
    assert!(dir.join(FILE_NAME).is_file());
    assert!(
        !dir.join(format!("{FILE_NAME}.tmp")).exists(),
        "原子写不留临时文件"
    );

    // 重启读取一致
    let runner = title_runner(&dir);
    let world = runner.app().world();
    assert_eq!(world.resources.expect::<Progress>(), &saved);
    assert!(world.resources.expect::<SaveSlot>().existed);

    // 损坏文件：备份为 .bak，以默认进度启动
    std::fs::write(dir.join(FILE_NAME), "Progress(version: 1, cleared: [").expect("写入");
    let runner = title_runner(&dir);
    let world = runner.app().world();
    assert!(dir.join(BACKUP_NAME).is_file());
    let fresh = world.resources.expect::<Progress>();
    assert!(fresh.cleared.is_empty());
    assert_eq!(fresh.unlocked.len(), 1, "默认只解锁初始卡牌");
    assert!(!world.resources.expect::<SaveSlot>().existed);

    // 版本不符同样走备份路径
    let mut old = saved.clone();
    old.version = Progress::VERSION + 1;
    save::write(&dir, &old).expect("写入");
    let _ = std::fs::remove_file(dir.join(BACKUP_NAME));
    let runner = title_runner(&dir);
    assert!(dir.join(BACKUP_NAME).is_file());
    assert!(
        runner
            .app()
            .world()
            .resources
            .expect::<Progress>()
            .cleared
            .is_empty()
    );
}

#[test]
fn audio_events_mapped() {
    let dir = temp_save(&format!("audio-{}", next_id()));
    let mut runner = title_runner(&dir);
    act(&mut runner, UiAction::StartLevel(lid("1-1")));
    assert_eq!(play_script(&mut runner, "1-1"), Some(Phase::Won));
    let log = runner.take_audio_log();
    let bank = runner.app().world().resources.expect::<AudioBank>();
    let mut keys = Vec::new();
    let mut music = 0;
    let mut crossfades = Vec::new();
    for req in &log {
        match req {
            AudioRequest::PlaySfx { clip, .. } => keys.extend(bank.key_of(*clip)),
            AudioRequest::PlayMusic { .. } => music += 1,
            AudioRequest::CrossfadeMusic { ticks, .. } => crossfades.push(*ticks),
            _ => {}
        }
    }
    for expected in [
        "sfx.shoot",
        "sfx.dew",
        "sfx.plant",
        "sfx.husk_down",
        "sfx.win",
    ] {
        assert!(keys.contains(&expected), "缺少 {expected}：{keys:?}");
    }
    // 首次播放（标题）为 PlayMusic，之后切到对局 / 结算 BGM 为交叉淡化（E-5，ADR-009）
    assert_eq!(music, 1, "标题 BGM 以 PlayMusic 开始（次数 {music}）");
    assert!(
        crossfades.len() >= 2
            && crossfades
                .iter()
                .all(|&t| t == rootguard_app::sfx::MUSIC_CROSSFADE_TICKS),
        "对局与结算 BGM 应交叉淡化：{crossfades:?}"
    );
}

/// 本帧音频日志中 `key` 音效的次数。
fn sfx_count(runner: &mut HeadlessRunner, key: &str) -> usize {
    let log = runner.take_audio_log();
    let bank = runner.app().world().resources.expect::<AudioBank>();
    log.iter()
        .filter(|req| {
            matches!(req, AudioRequest::PlaySfx { clip, .. } if bank.key_of(*clip) == Some(key))
        })
        .count()
}

/// 1-1 中路最右格放石籽荚，记录每个 tick 的 `sfx.bite` 次数与石籽荚被啃掉血的 tick。
/// `chill` 为真时让啃食中的敌人保持 Chill（测试直接写 `StatusEffects`，模拟 frost_pebble 命中）。
/// 返回 (音效 tick, 伤害 tick)。
fn bite_ticks(chill: bool) -> (Vec<u64>, Vec<u64>) {
    let mut runner = battle_runner("1-1", &[]);
    let mut sfx = Vec::new();
    let mut damage = Vec::new();
    let mut last_hp = None;
    for _ in 0..6000 {
        let s = state(&runner).expect("对局");
        if s.level_tick == 60 {
            send(&mut runner, UiAction::SelectCard(0));
            send(&mut runner, UiAction::PlaceAt { row: 2, col: 8 });
        }
        if chill {
            let world = runner.app_mut().world_mut();
            for (walker, status) in world.entities.query_mut::<(&Walker, &mut StatusEffects)>() {
                if matches!(walker.state, WalkState::Eating(_)) {
                    status.chill_ticks = 600;
                }
            }
        }
        runner.frame();
        let tick = state(&runner).expect("对局").level_tick;
        for _ in 0..sfx_count(&mut runner, "sfx.bite") {
            sfx.push(tick);
        }
        let hp = runner
            .app()
            .world()
            .query::<(&Health, &DefenderRef)>()
            .iter()
            .map(|(h, _)| h.hp)
            .next();
        if let (Some(before), Some(now)) = (last_hp, hp)
            && now < before
        {
            damage.push(tick);
        }
        last_hp = hp;
        if damage.len() >= 4 {
            break;
        }
    }
    (sfx, damage)
}

/// 每次啃食伤害之前 `0 ..= window` tick 内恰有一次 `sfx.bite`，且没有多余的音效。
fn assert_bites_in_phase(sfx: &[u64], damage: &[u64], window: u64) {
    assert!(damage.len() >= 3, "应观察到多次啃食伤害（{damage:?}）");
    for &d in damage {
        let near = sfx.iter().filter(|&&s| s <= d && d - s <= window).count();
        assert_eq!(
            near, 1,
            "伤害 tick {d} 之前 {window} tick 内应恰有一次音效（音效 {sfx:?}）"
        );
    }
    assert!(
        sfx.len() <= damage.len() + 1,
        "音效 {sfx:?} 多于伤害 {damage:?}"
    );
}

#[test]
fn bite_sfx_on_eat_hit() {
    // eat_hit 在 eat 片段末帧，片段相位由 Biter 计时器驱动（负责人裁决 2026-10-15（四））：
    // 音效与啃食伤害同频同相，偏差 ≤ 末帧长度 10 tick；Chill 下间隔翻倍，偏差 ≤ 20 tick。
    // 默认占位模式下同样触发：音效不随渲染模式变化（ADR-010）。
    let (sfx, damage) = bite_ticks(false);
    assert_bites_in_phase(&sfx, &damage, 10);
    let (sfx, damage) = bite_ticks(true);
    assert_bites_in_phase(&sfx, &damage, 20);
    let app = rootguard_app::build_app_with(TitleScene);
    let world = app.world();
    let husk = world
        .resources
        .expect::<EnemyRegistry>()
        .iter()
        .find(|(id, _)| id.as_str() == "husk")
        .map(|(_, def)| def.bite_interval_ticks)
        .expect("husk");
    let chilled = u64::from(
        husk * world
            .resources
            .expect::<Rules>()
            .status
            .chill_bite_interval_factor,
    );
    let gaps: Vec<u64> = damage.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        gaps.iter().all(|&g| g == chilled),
        "Chill 下啃食间隔应为 {chilled}（{gaps:?}）"
    );
}

#[test]
fn i18n_tables_complete() {
    let dir = temp_save(&format!("i18n-{}", next_id()));
    let runner = title_runner(&dir);
    let world = runner.app().world();
    let i18n = world.resources.expect::<I18n>();
    let (only_zh, only_en) = i18n.key_diff();
    assert!(
        only_zh.is_empty() && only_en.is_empty(),
        "中英文表 key 不一致：{only_zh:?} / {only_en:?}"
    );
    for (id, _) in world.resources.expect::<DefenderRegistry>().iter() {
        for suffix in ["name", "desc"] {
            assert!(
                i18n.has(&format!("defender.{id}.{suffix}")),
                "缺少 defender.{id}.{suffix}"
            );
        }
    }
    for (id, _) in world.resources.expect::<EnemyRegistry>().iter() {
        for suffix in ["name", "desc"] {
            assert!(
                i18n.has(&format!("enemy.{id}.{suffix}")),
                "缺少 enemy.{id}.{suffix}"
            );
        }
    }
}

/// SPEC 11.6 / E-1：默认中文时对局 HUD 直接显示中文（不再回退英文），且 CJK 字体可解析。
#[test]
fn chinese_hud_when_lang_zh() {
    let dir = temp_save(&format!("zh-{}", next_id()));
    let mut runner = title_runner(&dir);
    act(&mut runner, UiAction::SetLanguage(Lang::Zh));
    act(&mut runner, UiAction::StartLevel(lid("1-1")));
    assert_eq!(top(&runner), Some("Battle"));
    runner.frame();
    let texts: Vec<String> = runner
        .last_draw_list()
        .iter()
        .filter_map(|c| match &c.kind {
            DrawKind::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|t| t.starts_with("关卡 1-1")),
        "HUD 应显示中文关卡名：{texts:?}"
    );
    assert!(texts.iter().any(|t| t == "准备……"), "Intro 横幅应为中文");
    let font = rootguard_app::cjk_font_path().expect("CJK 字体应随资产提供");
    assert!(font.is_file());
}
