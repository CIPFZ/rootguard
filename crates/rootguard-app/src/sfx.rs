//! 音效与音乐（SPEC 13）：对局事件 → `AudioQueue`；场景切换时更换循环 BGM。
//!
//! 全部音频由 `scripts/gen_audio.py` 程序化合成（`assets/SOURCES.md`）。

use std::collections::BTreeMap;

use rootguard_core::events::{
    BattleEnded, DefenderPlaced, DefenderRemoved, DewCollected, EnemyDied, HoseTriggered, Outcome,
    ProjectileFired, ProjectileHit, RemoveCause, VolumeBus, WaveStarted,
};
use rootguard_core::resources::Progress;
use trellia::prelude::*;

/// 音效 key 与文件（SPEC 13 表）。
pub const SFX: [(&str, &str); 11] = [
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
];

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

/// 切换循环 BGM；与当前曲目相同时不重新开始。
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
    state.music = Some(track);
    if let Some(queue) = world.resources.get_mut::<AudioQueue>() {
        queue.play_music(handle, true);
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
    keys.extend(drain::<ProjectileFired>(world).iter().map(|_| "sfx.shoot"));
    keys.extend(drain::<ProjectileHit>(world).iter().map(|h| {
        if h.armored {
            "sfx.hit_armor"
        } else {
            "sfx.hit"
        }
    }));
    keys.extend(drain::<DefenderPlaced>(world).iter().map(|_| "sfx.plant"));
    keys.extend(
        drain::<DefenderRemoved>(world)
            .iter()
            .filter(|r| r.cause == RemoveCause::Shovel)
            .map(|_| "sfx.dig"),
    );
    keys.extend(drain::<DewCollected>(world).iter().map(|_| "sfx.dew"));
    keys.extend(drain::<EnemyDied>(world).iter().map(|_| "sfx.husk_down"));
    keys.extend(
        drain::<WaveStarted>(world)
            .iter()
            .filter(|w| w.flag)
            .map(|_| "sfx.flag_wave"),
    );
    keys.extend(drain::<HoseTriggered>(world).iter().map(|_| "sfx.hose"));
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
