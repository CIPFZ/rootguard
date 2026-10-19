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
    // ART-SPEC 8.4：每关一句园丁独白与结局一句（中英 key 一致已在上面检查）
    for level in world.resources.expect::<LevelRegistry>().iter() {
        let key = rootguard_app::hud::intro_key(level.id.as_str());
        assert!(i18n.has(&key), "缺少 {key}");
    }
    assert!(i18n.has("ending.line"), "缺少 ending.line");
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
    // ART-SPEC 8.4：Intro 横幅是该关的园丁独白（LORE §4），替换“准备”
    assert!(
        texts
            .iter()
            .any(|t| t == "泉水停了。菜地那边有东西在动，我先把石籽荚请出来。"),
        "Intro 横幅应为 1-1 的中文独白：{texts:?}"
    );
    let font = rootguard_app::cjk_font_path().expect("CJK 字体应随资产提供");
    assert!(font.is_file());
}

/// ART-SPEC 0 / ADR-014：设置新增 `skin_name` 前写下的存档（无该字段）照常读取，不走备份路径（v1，按裁决十迁移）。
#[test]
fn save_without_skin_name_loads() {
    let dir = temp_save(&format!("save-compat-{}", next_id()));
    std::fs::create_dir_all(&dir).expect("目录");
    std::fs::write(
        dir.join(FILE_NAME),
        r#"Progress(version: 1, cleared: ["1-1"], unlocked: ["pebble_pod", "dewcup"],
            settings: (sfx: 0.5, music: 0.75, lang: En, skin: Skinned))"#,
    )
    .expect("写入");
    let runner = title_runner(&dir);
    let world = runner.app().world();
    assert!(!dir.join(BACKUP_NAME).exists(), "旧存档不应被视为损坏");
    let p = world.resources.expect::<Progress>();
    assert!(p.cleared.contains(&lid("1-1")));
    assert_eq!(p.settings.skin, rootguard_core::events::SkinMode::Skinned);
    // 缺省的皮肤为 garden（裁决八第 3 节第 7 项：Skinned 默认 garden）
    assert_eq!(
        p.settings.skin_name,
        rootguard_core::events::SkinName::Garden
    );
}

/// 回放某关脚本，返回全部音效 key 的出现次数（默认占位模式：音效不随渲染模式变化，ADR-010）。
fn replay_sfx(level: &str) -> std::collections::BTreeMap<String, usize> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tests/scripts/{level}.ron"));
    let script = Script::parse(&std::fs::read_to_string(path).expect("脚本")).expect("解析");
    let scene = rootguard_app::BattleScene::new(script.level, script.cards.clone(), None);
    let mut runner = common::runner_with(scene, &format!("sfx-{}", next_id()));
    let mut replay = Replay::new(script);
    runner.frame();
    let mut counts = std::collections::BTreeMap::new();
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
        let log = runner.take_audio_log();
        let bank = runner.app().world().resources.expect::<AudioBank>();
        for req in &log {
            if let AudioRequest::PlaySfx { clip, .. } = req
                && let Some(k) = bank.key_of(*clip)
            {
                *counts.entry(k.to_owned()).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// ART-SPEC 12 / ADR-016 第 7 节：美术阶段新增音效由对应事件或片段事件帧触发。
#[test]
fn art_phase_sfx_mapped() {
    let counts = replay_sfx("1-10");
    // 1-10 的卡组没有咬夹草、风铃花与葫芦壳（snap / gulp / gust / shell_crack 见 constructed_sfx_mapped）
    for key in [
        "sfx.drip",
        "sfx.lantern",
        "sfx.wood_crack",
        "sfx.wilt",
        "sfx.shoot_frost",
        "sfx.hit_frost",
        "sfx.prick",
        "sfx.tuber_burst",
        "sfx.ember_burst",
        "sfx.twig_snap",
        "sfx.pot_break",
        "sfx.gourd_break",
        "sfx.vault",
        "sfx.seeds_spill",
        "sfx.thud",
        "sfx.sprout_down",
    ] {
        assert!(counts.contains_key(key), "1-10 中应有 {key}（{counts:?}）");
    }
    // 1-10 只用冰石：普通发射与命中不应出现
    assert!(!counts.contains_key("sfx.shoot") && !counts.contains_key("sfx.hit"));
    // 第 4 次生产才响铃（每只日灯笼约每 4 × 1440 tick 一次），远少于滴落
    assert!(counts["sfx.lantern"] * 4 <= counts["sfx.drip"]);
}

/// ADR-016 第 7 节：咬夹草吞食 `sfx.snap` 与 snap 片段末帧 `sfx.gulp`、风铃花 `sfx.gust`、葫芦壳被吃光
/// `sfx.shell_crack`（测试把葫芦壳生命改为 1 只为尽快被吃光）。
#[test]
fn constructed_sfx_mapped() {
    use rootguard_core::components::OverlayOf;
    use rootguard_core::resources::{BattleState, CardTray};
    let mut runner = battle_runner("1-10", &["snapmaw", "gustbloom", "barkwall", "gourdshell"]);
    while state(&runner).is_some_and(|s| s.phase != Phase::Playing) {
        runner.frame();
    }
    let refill = |runner: &mut HeadlessRunner| {
        let world = runner.app_mut().world_mut();
        world.resources.expect_mut::<BattleState>().dew = 9990;
        for c in &mut world.resources.expect_mut::<CardTray>().cards {
            c.cooldown.remaining = 0;
        }
    };
    let mut placed = 0;
    let mut counts = std::collections::BTreeMap::new();
    for _ in 0..8000 {
        refill(&mut runner);
        // 第 0 ～ 2 行第 7 列咬夹草，第 3 ～ 4 行第 7 列树皮墙 + 葫芦壳；有敌人后在第 1 行第 5 列放风铃花
        let plan: [(usize, u8, u8); 8] = [
            (0, 0, 7),
            (0, 1, 7),
            (0, 2, 7),
            (2, 3, 7),
            (2, 4, 7),
            (3, 3, 7),
            (3, 4, 7),
            (1, 1, 5),
        ];
        let enemies = runner
            .app()
            .world()
            .query::<&rootguard_core::components::EnemyRef>()
            .iter()
            .count();
        if placed < plan.len() && (placed < 7 || enemies > 0) {
            let (slot, row, col) = plan[placed];
            send(&mut runner, UiAction::SelectCard(slot));
            send(&mut runner, UiAction::PlaceAt { row, col });
            placed += 1;
        }
        runner.frame();
        {
            let world = runner.app_mut().world_mut();
            for (h, _) in world.entities.query_mut::<(&mut Health, &OverlayOf)>() {
                h.hp = h.hp.min(1);
            }
        }
        let log = runner.take_audio_log();
        let bank = runner.app().world().resources.expect::<AudioBank>();
        for req in &log {
            if let AudioRequest::PlaySfx { clip, .. } = req
                && let Some(k) = bank.key_of(*clip)
            {
                *counts.entry(k.to_owned()).or_insert(0) += 1;
            }
        }
        if ["sfx.snap", "sfx.gulp", "sfx.gust", "sfx.shell_crack"]
            .iter()
            .all(|k| counts.contains_key(*k))
        {
            break;
        }
    }
    for key in ["sfx.snap", "sfx.gulp", "sfx.gust", "sfx.shell_crack"] {
        assert!(counts.contains_key(key), "应有 {key}（{counts:?}）");
    }
    assert_eq!(
        counts["sfx.snap"], counts["sfx.gulp"],
        "每次吞食一声 snap、一声 gulp"
    );
}

/// 读取 OpenType `cmap`（格式 4 / 12）中的全部码位（只为测试字体覆盖，不是游戏代码）。
fn font_codepoints(bytes: &[u8]) -> std::collections::BTreeSet<u32> {
    let u16_at = |o: usize| u32::from(u16::from_be_bytes([bytes[o], bytes[o + 1]]));
    let u32_at =
        |o: usize| u32::from_be_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    let tables = u16_at(4) as usize;
    let cmap = (0..tables)
        .map(|i| 12 + 16 * i)
        .find(|&r| &bytes[r..r + 4] == b"cmap")
        .map(|r| u32_at(r + 8) as usize)
        .expect("cmap 表");
    let mut out = std::collections::BTreeSet::new();
    for i in 0..u16_at(cmap + 2) as usize {
        let sub = cmap + u32_at(cmap + 4 + 8 * i + 4) as usize;
        match u16_at(sub) {
            4 => {
                let segs = u16_at(sub + 6) as usize / 2;
                let ends = sub + 14;
                let starts = ends + 2 * segs + 2;
                let deltas = starts + 2 * segs;
                let ranges = deltas + 2 * segs;
                for s in 0..segs {
                    let (start, end) = (u16_at(starts + 2 * s), u16_at(ends + 2 * s));
                    let range = u16_at(ranges + 2 * s);
                    for c in start..=end.min(0xFFFE) {
                        let glyph = if range == 0 {
                            (c + u16_at(deltas + 2 * s)) & 0xFFFF
                        } else {
                            let at = ranges + 2 * s + range as usize + 2 * (c - start) as usize;
                            u16_at(at)
                        };
                        if glyph != 0 {
                            out.insert(c);
                        }
                    }
                }
            }
            12 => {
                for g in 0..u32_at(sub + 12) as usize {
                    let rec = sub + 16 + 12 * g;
                    out.extend(u32_at(rec)..=u32_at(rec + 4));
                }
            }
            _ => {}
        }
    }
    out
}

/// SPEC 11.6 / E-1：随资产提供的 CJK 子集字体覆盖文本表中出现的每个字符（新增文本后忘了重跑
/// `scripts/subset_font.py` 会显示成方框）。
#[test]
fn cjk_font_covers_text_tables() {
    let font = std::fs::read(rootguard_app::cjk_font_path().expect("CJK 字体")).expect("读取字体");
    let have = font_codepoints(&font);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/i18n");
    let mut missing = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(dir).expect("i18n 目录") {
        let text = std::fs::read_to_string(entry.expect("文件").path()).expect("读取");
        for c in text
            .chars()
            .filter(|c| !c.is_control() && !c.is_whitespace())
        {
            if !have.contains(&u32::from(c)) {
                missing.insert(c);
            }
        }
    }
    assert!(
        missing.is_empty(),
        "字体缺少 {missing:?}：请运行 scripts/subset_font.py 并更新 assets/SOURCES.md"
    );
}

/// 裁决十 / SPEC v1.3 12.1：存档 v1 → v2 迁移。v1 存档（0.1.0 / 0.2.0）一律改为 Skinned + Garden，其余照旧、
/// 不走备份；v2 存档照常读取（玩家在 v2 里主动选的占位模式保留）；更高的版本仍走备份路径。新存档默认 Skinned + Garden。
#[test]
fn save_v1_migrates_to_skinned_garden() {
    use rootguard_core::events::{SkinMode, SkinName};
    let load = |text: &str| {
        let dir = temp_save(&format!("migrate-{}", next_id()));
        std::fs::create_dir_all(&dir).expect("目录");
        std::fs::write(dir.join(FILE_NAME), text).expect("写入");
        let (p, ok) = save::load(&dir, None);
        (p, ok, dir.join(BACKUP_NAME).exists())
    };
    let v1 = r#"Progress(version: 1, cleared: ["1-1", "1-2"], unlocked: ["pebble_pod", "dewcup"],
        settings: (sfx: 0.5, music: 0.75, lang: En, skin: Placeholder, skin_name: Default))"#;
    let (p, ok, bak) = load(v1);
    assert!(ok && !bak, "v1 存档应迁移而不是备份");
    assert_eq!(p.version, Progress::VERSION);
    assert_eq!(p.settings.skin, SkinMode::Skinned);
    assert_eq!(p.settings.skin_name, SkinName::Garden);
    assert_eq!(p.cleared.len(), 2, "进度不变");
    assert!(
        (p.settings.sfx - 0.5).abs() < 1e-6 && p.settings.lang == Lang::En,
        "其他设置不变"
    );

    let v2 = r#"Progress(version: 2, cleared: [], unlocked: ["pebble_pod"],
        settings: (sfx: 1.0, music: 1.0, lang: Zh, skin: Placeholder, skin_name: Default))"#;
    let (p, ok, bak) = load(v2);
    assert!(ok && !bak);
    assert_eq!(
        p.settings.skin,
        SkinMode::Placeholder,
        "v2 中主动选的占位模式保留"
    );
    assert_eq!(p.settings.skin_name, SkinName::Default);

    let (_, ok, bak) = load(&v2.replace("version: 2", "version: 3"));
    assert!(!ok && bak, "未知版本仍走备份路径");

    let fresh = save::default_progress(None);
    assert_eq!(fresh.version, 2);
    assert_eq!(
        (fresh.settings.skin, fresh.settings.skin_name),
        (SkinMode::Skinned, SkinName::Garden)
    );
}
