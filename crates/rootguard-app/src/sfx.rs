//! 音效与音乐（SPEC 13）：对局事件 → `AudioQueue`；场景切换时更换循环 BGM。
//!
//! 全部音频由 `scripts/gen_audio.py` 程序化合成（`assets/SOURCES.md`）。

use std::collections::{BTreeMap, BTreeSet};

use rootguard_core::components::{DefenderRef, EnemyRef, Producer, ProjectileBody};
use rootguard_core::data::ArmorKind;
use rootguard_core::events::{
    BattleEnded, DamageSource, DefenderPlaced, DefenderRemoved, DewCollected, DewSource,
    DewSpawned, EnemyArmorBroken, EnemyDamaged, EnemyDied, HoseTriggered, Outcome, ProjectileFired,
    ProjectileHit, RemoveCause, VolumeBus, WaveStarted,
};
use rootguard_core::resources::Progress;
use trellia::prelude::*;

/// 音效 key 与文件（SPEC 13 表；第 13 项起为 ART-SPEC 12 美术阶段新增）。
pub const SFX: [(&str, &str); 32] = [
    ("sfx.shoot", "audio/sfx_shoot.ogg"),
    ("sfx.hit", "audio/sfx_hit.ogg"),
    ("sfx.hit_armor", "audio/sfx_hit_armor.ogg"),
    ("sfx.plant", "audio/sfx_plant.ogg"),
    ("sfx.dig", "audio/sfx_dig.ogg"),
    ("sfx.dew", "audio/sfx_dew.ogg"),
    ("sfx.husk_down", "audio/sfx_husk_down.ogg"),
    ("sfx.flag_wave", "audio/sfx_flag_wave.ogg"),
    ("sfx.hose", "audio/sfx_hose.ogg"),
    ("sfx.win", "audio/sfx_win.ogg"),
    ("sfx.lose", "audio/sfx_lose.ogg"),
    ("sfx.bite", "audio/sfx_bite.ogg"),
    ("sfx.drip", "audio/sfx_drip.ogg"),
    ("sfx.wood_crack", "audio/sfx_wood_crack.ogg"),
    ("sfx.shoot_frost", "audio/sfx_shoot_frost.ogg"),
    ("sfx.hit_frost", "audio/sfx_hit_frost.ogg"),
    ("sfx.prick", "audio/sfx_prick.ogg"),
    ("sfx.tuber_burst", "audio/sfx_tuber_burst.ogg"),
    ("sfx.ember_burst", "audio/sfx_ember_burst.ogg"),
    ("sfx.shell_crack", "audio/sfx_shell_crack.ogg"),
    ("sfx.snap", "audio/sfx_snap.ogg"),
    ("sfx.gulp", "audio/sfx_gulp.ogg"),
    ("sfx.gust", "audio/sfx_gust.ogg"),
    ("sfx.lantern", "audio/sfx_lantern.ogg"),
    ("sfx.wilt", "audio/sfx_wilt.ogg"),
    ("sfx.twig_snap", "audio/sfx_twig_snap.ogg"),
    ("sfx.pot_break", "audio/sfx_pot_break.ogg"),
    ("sfx.vault", "audio/sfx_vault.ogg"),
    ("sfx.seeds_spill", "audio/sfx_seeds_spill.ogg"),
    ("sfx.thud", "audio/sfx_thud.ogg"),
    ("sfx.gourd_break", "audio/sfx_gourd_break.ogg"),
    ("sfx.sprout_down", "audio/sfx_sprout_down.ogg"),
];

/// 片段事件帧 → 音效（ART-SPEC 3.2）：呈现层以引擎 `AnimationEvent` 发出（ADR-010 / ADR-016 第 7 节）。
const ANIM_SFX: [(&str, &str); 5] = [
    (EAT_HIT_EVENT, "sfx.bite"),
    ("gulp", "sfx.gulp"),
    ("thud", "sfx.thud"),
    ("prick", "sfx.prick"),
    ("vault", "sfx.vault"),
];

/// 皮肤 eat 片段的啃食事件帧（SPEC 12.3），由呈现层以引擎 `AnimationEvent` 发出（ADR-010）。
pub const EAT_HIT_EVENT: &str = "eat_hit";

/// 同一 tick 同一音效最多触发次数（SPEC 13）。
const MAX_SAME_SFX_PER_TICK: usize = 2;

/// 循环 BGM（SPEC 13：Title / Battle / Outcome 各一首）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Track {
    /// 标题与菜单。
    Title,
    /// 对局。
    Battle,
    /// 结算。
    Outcome,
}

impl Track {
    fn path(self) -> &'static str {
        match self {
            Track::Title => "audio/bgm_title.ogg",
            Track::Battle => "audio/bgm_battle.ogg",
            Track::Outcome => "audio/bgm_outcome.ogg",
        }
    }
}

/// 资源：已加载的音频句柄。
pub struct AudioBank {
    sfx: BTreeMap<&'static str, Handle<AudioClip>>,
    music: BTreeMap<&'static str, Handle<AudioClip>>,
}

impl AudioBank {
    /// 音效句柄。
    #[must_use]
    pub fn sfx(&self, key: &str) -> Option<Handle<AudioClip>> {
        self.sfx.get(key).copied()
    }

    /// 音效 key（`audio_log` 断言用）：句柄 → key。
    #[must_use]
    pub fn key_of(&self, handle: Handle<AudioClip>) -> Option<&'static str> {
        self.sfx
            .iter()
            .chain(self.music.iter())
            .find(|(_, h)| **h == handle)
            .map(|(k, _)| *k)
    }
}

/// 资源：当前 BGM 与上一帧的 tick 计数。
#[derive(Default)]
struct AudioState {
    music: Option<Track>,
    last_tick: u64,
    /// 飞行中的弹丸：实体位 → 是否冰石（命中音效 `sfx.hit_frost`）。
    frost_projectiles: BTreeMap<u64, bool>,
    /// 上一帧由“只读不取走”的事件生成过的音效键（双缓冲去重）。
    read_keys: BTreeSet<String>,
}

fn load(world: &mut World) -> Result<AudioBank, AssetError> {
    let mut add = |rel: &'static str| load_audio_into(world, rel);
    let mut sfx = BTreeMap::new();
    for (key, rel) in SFX {
        sfx.insert(key, add(rel)?);
    }
    let mut music = BTreeMap::new();
    for track in [Track::Title, Track::Battle, Track::Outcome] {
        music.insert(track.path(), add(track.path())?);
    }
    Ok(AudioBank { sfx, music })
}

/// 场景切换时 BGM 交叉淡化的时长（tick，E-5）。
pub const MUSIC_CROSSFADE_TICKS: u32 = 60;

/// 切换循环 BGM；与当前曲目相同时不重新开始。首次播放直接开始（`PlayMusic`），
/// 之后的切换在 [`MUSIC_CROSSFADE_TICKS`] 内交叉淡化（`CrossfadeMusic`，SPEC 15 E-5，ADR-009）。
pub fn play_music(world: &mut World, track: Track) {
    let Some(handle) = world
        .resources
        .get::<AudioBank>()
        .and_then(|b| b.music.get(track.path()).copied())
    else {
        return;
    };
    let Some(state) = world.resources.get_mut::<AudioState>() else {
        return;
    };
    if state.music == Some(track) {
        return;
    }
    let first = state.music.replace(track).is_none();
    if let Some(queue) = world.resources.get_mut::<AudioQueue>() {
        if first {
            queue.play_music(handle, true);
        } else {
            queue.crossfade_music(handle, MUSIC_CROSSFADE_TICKS);
        }
    }
}

/// 把设置中的音量应用到总线。
pub fn apply_volumes(world: &mut World) {
    let Some(settings) = world.resources.get::<Progress>().map(|p| p.settings) else {
        return;
    };
    if let Some(queue) = world.resources.get_mut::<AudioQueue>() {
        queue.set_bus_volume(AudioBus::Sfx, settings.sfx);
        queue.set_bus_volume(AudioBus::Music, settings.music);
    }
}

/// 模拟层总线 → 引擎总线（ADR-002 第 6 条）。
#[must_use]
pub fn engine_bus(bus: VolumeBus) -> AudioBus {
    match bus {
        VolumeBus::Sfx => AudioBus::Sfx,
        VolumeBus::Music => AudioBus::Music,
    }
}

fn drain<T: Event>(world: &mut World) -> Vec<T> {
    world.events.get_mut::<T>().drain().collect()
}

/// 冰石弹丸 id（`sfx.shoot_frost` / `sfx.hit_frost`）。
const FROST_PROJECTILE: &str = "frost_pebble";

/// 不能取走（呈现层也要读）的事件生成的音效：同一事件在本帧与下一帧都可读，记住上一帧的键去重
/// （ADR-016 第 7 节）。吞食 `sfx.snap`（4.10）；护甲破碎（5.2 / 5.3 / 5.8）；生产 `sfx.drip` /
/// 日灯笼第 `bonus_every` 次 `sfx.lantern`（4.2 / 4.12）。
fn read_once_sfx(world: &mut World) -> Vec<&'static str> {
    let mut found: Vec<(String, &'static str)> = Vec::new();
    for d in world.events.get::<EnemyDamaged>().iter() {
        if d.source == DamageSource::Chomp {
            found.push((format!("snap:{}", d.entity.to_bits().get()), "sfx.snap"));
        }
    }
    for b in world.events.get::<EnemyArmorBroken>().iter() {
        let enemy = world.get::<EnemyRef>(b.entity).map(|r| r.id);
        let key = match (enemy.as_ref().map(|i| i.as_str()), b.kind) {
            (Some("clay_husk"), _) => "sfx.pot_break",
            (_, ArmorKind::Helm) => "sfx.twig_snap",
            (_, ArmorKind::Drum) => "sfx.gourd_break",
            (_, ArmorKind::Shield) => continue,
        };
        found.push((format!("armor:{}", b.entity.to_bits().get()), key));
    }
    for d in world.events.get::<DewSpawned>().iter() {
        let DewSource::Producer(from) = d.source else {
            continue;
        };
        // 同一次生产的额外产量也是一个事件：按生产者去重
        let bonus = world.get::<Producer>(from).is_some_and(|p| {
            p.bonus_every > 0 && p.produced > 0 && p.produced.is_multiple_of(p.bonus_every)
        }) && world.get::<DefenderRef>(from).is_some();
        let key = if bonus { "sfx.lantern" } else { "sfx.drip" };
        found.push((format!("dew:{}", from.to_bits().get()), key));
    }
    let alive: BTreeSet<u64> = world
        .query::<(Entity, &ProjectileBody)>()
        .iter()
        .map(|(e, _)| e.to_bits().get())
        .collect();
    let Some(state) = world.resources.get_mut::<AudioState>() else {
        return Vec::new();
    };
    // 飞出画面而未命中的弹丸
    state.frost_projectiles.retain(|k, _| alive.contains(k));
    let mut keys = Vec::new();
    let mut now = BTreeSet::new();
    for (k, sfx) in found {
        if !state.read_keys.contains(&k) && now.insert(k.clone()) {
            keys.push(sfx);
        }
        now.insert(k);
    }
    state.read_keys = now;
    keys
}

/// `Update`：取走本帧的音效事件并写入 `AudioQueue`（SPEC 13）。本系统是这些事件的唯一消费者。
fn sfx_system(world: &mut World) {
    let tick = world.resources.expect::<Time>().fixed_tick;
    let ticks = {
        let Some(state) = world.resources.get_mut::<AudioState>() else {
            return;
        };
        let n = tick.saturating_sub(state.last_tick);
        state.last_tick = tick;
        usize::try_from(n.max(1)).unwrap_or(usize::MAX)
    };
    let mut keys: Vec<&'static str> = Vec::new();
    // 发射：冰石带高频“叮”（4.4）；记住弹丸种类供命中使用
    let fired = drain::<ProjectileFired>(world);
    let mut frost: Vec<(u64, bool)> = Vec::new();
    for f in &fired {
        let is_frost = f.id.as_str() == FROST_PROJECTILE;
        keys.push(if is_frost {
            "sfx.shoot_frost"
        } else {
            "sfx.shoot"
        });
        frost.push((f.entity.to_bits().get(), is_frost));
    }
    let hits = drain::<ProjectileHit>(world);
    {
        let state = world.resources.expect_mut::<AudioState>();
        state.frost_projectiles.extend(frost);
        for h in &hits {
            let is_frost = state
                .frost_projectiles
                .remove(&h.entity.to_bits().get())
                .unwrap_or(false);
            keys.push(if h.armored {
                "sfx.hit_armor"
            } else if is_frost {
                "sfx.hit_frost"
            } else {
                "sfx.hit"
            });
        }
    }
    keys.extend(drain::<DefenderPlaced>(world).iter().map(|_| "sfx.plant"));
    // 防御者离场（4.3 / 4.7 / 4.8 / 4.9 / 4.11 / 4.13）
    for r in drain::<DefenderRemoved>(world) {
        keys.push(match (r.cause, r.id.as_str()) {
            (RemoveCause::Shovel, _) => "sfx.dig",
            (RemoveCause::Killed, "barkwall") => "sfx.wood_crack",
            (RemoveCause::Killed, "gourdshell") => "sfx.shell_crack",
            (RemoveCause::Killed, _) => "sfx.wilt",
            (RemoveCause::Consumed, "tuberblast") => "sfx.tuber_burst",
            (RemoveCause::Consumed, "emberbulb") => "sfx.ember_burst",
            (RemoveCause::Consumed, "gustbloom") => "sfx.gust",
            (RemoveCause::Consumed, _) => continue,
        });
    }
    keys.extend(drain::<DewCollected>(world).iter().map(|_| "sfx.dew"));
    // 死亡：小芽怪更轻（5.9），背篓倒下撒种子（5.7）
    for d in drain::<EnemyDied>(world) {
        match d.id.as_str() {
            "sproutling" => keys.push("sfx.sprout_down"),
            "carrier_husk" => keys.extend(["sfx.husk_down", "sfx.seeds_spill"]),
            _ => keys.push("sfx.husk_down"),
        }
    }
    keys.extend(
        drain::<WaveStarted>(world)
            .iter()
            .filter(|w| w.flag)
            .map(|_| "sfx.flag_wave"),
    );
    keys.extend(drain::<HoseTriggered>(world).iter().map(|_| "sfx.hose"));
    keys.extend(drain::<AnimationEvent>(world).iter().filter_map(|a| {
        ANIM_SFX
            .iter()
            .find(|(event, _)| *event == a.event)
            .map(|(_, key)| *key)
    }));
    keys.extend(read_once_sfx(world));
    keys.extend(drain::<BattleEnded>(world).iter().map(|e| match e.outcome {
        Outcome::Won => "sfx.win",
        Outcome::Lost => "sfx.lose",
    }));
    if keys.is_empty() {
        return;
    }
    let Some(bank) = world.resources.get::<AudioBank>() else {
        return;
    };
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut handles = Vec::new();
    for key in keys {
        let n = counts.entry(key).or_default();
        if *n >= MAX_SAME_SFX_PER_TICK * ticks {
            continue;
        }
        *n += 1;
        if let Some(h) = bank.sfx(key) {
            handles.push(h);
        }
    }
    if let Some(queue) = world.resources.get_mut::<AudioQueue>() {
        for h in handles {
            queue.play_sfx(h);
        }
    }
}

/// SPEC 3.3 `SfxPlugin`：加载全部音频（失败记启动错误），注册事件 → 音效系统，启动时应用音量。
pub struct SfxPlugin;

impl Plugin for SfxPlugin {
    fn name(&self) -> &'static str {
        "SfxPlugin"
    }

    fn build(&self, app: &mut App) {
        match load(app.world_mut()) {
            Ok(bank) => {
                app.insert_resource(bank);
            }
            Err(e) => app.record_startup_error(e),
        }
        app.insert_resource(AudioState::default())
            .add_system(Stage::Startup, apply_volumes)
            .add_system(Stage::Update, sfx_system);
    }
}
