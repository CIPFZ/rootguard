//! 确定性快照（SPEC 16.2 `determinism_golden_1_4`）：把全部模拟状态按实体 id 升序写成字节，
//! 小端序、与平台无关。`rootguard-app` 把它与引擎 `HeadlessRunner::snapshot()` 拼接后比对 golden。

use trellia_core::{Entity, Timer, Transform2D, World};

use crate::components::{
    Armor, BattleTag, Biter, Blast, Cell, ChompState, Chomper, DefenderRef, DewPickup, EnemyRef,
    Gust, Health, Mine, OverlayOf, Producer, ProjectileBody, Shielded, Shooter, Spikes, Sprinter,
    StatusEffects, Vaulter, WalkState, Walker,
};
use crate::resources::{BattleState, CardTray, LaneState, Phase, WaveState};

struct Out(Vec<u8>);

impl Out {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn bool(&mut self, v: bool) {
        self.0.push(u8::from(v));
    }
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend(v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend(v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend(v.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.u8(u8::try_from(s.len()).unwrap_or(u8::MAX));
        self.0.extend(s.as_bytes());
    }
    fn entity(&mut self, e: Entity) {
        self.u64(e.to_bits().get());
    }
    fn timer(&mut self, t: Timer) {
        self.u32(t.remaining);
        self.u32(t.duration);
        self.bool(t.repeating);
    }
    /// 写可选组件：存在标记 + 内容。
    fn opt<T>(&mut self, tag: u8, v: Option<T>, f: impl FnOnce(&mut Self, T)) {
        if let Some(v) = v {
            self.u8(tag);
            f(self, v);
        }
    }
}

/// 写出全部对局状态：资源（`BattleState` / `CardTray` / `WaveState` / `LaneState`）+ 每个
/// `BattleTag` 实体的模拟组件。
#[must_use]
pub fn write(world: &World) -> Vec<u8> {
    let mut out = Out(Vec::new());
    write_resources(world, &mut out);
    let mut entities: Vec<Entity> = world
        .query::<(Entity, &BattleTag)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    entities.sort_by_key(|e| e.to_bits());
    out.u32(u32::try_from(entities.len()).unwrap_or(u32::MAX));
    for e in entities {
        write_entity(world, e, &mut out);
    }
    out.0
}

fn write_resources(world: &World, out: &mut Out) {
    if let Some(s) = world.resources.get::<BattleState>() {
        out.str(s.level.as_str());
        out.u64(s.level_tick);
        out.i32(s.dew);
        out.u8(match s.phase {
            Phase::Intro => 0,
            Phase::Playing => 1,
            Phase::Won => 2,
            Phase::Lost => 3,
        });
        out.u32(
            s.selected_card
                .map_or(u32::MAX, |i| u32::try_from(i).unwrap_or(u32::MAX)),
        );
        out.bool(s.shovel_mode);
    }
    if let Some(tray) = world.resources.get::<CardTray>() {
        for c in &tray.cards {
            out.str(c.defender.as_str());
            out.timer(c.cooldown);
            out.bool(c.ready);
        }
    }
    if let Some(w) = world.resources.get::<WaveState>() {
        out.u32(u32::try_from(w.next_wave).unwrap_or(u32::MAX));
        out.timer(w.wave_timer);
        out.i32(w.wave_hp_total);
        out.i32(w.wave_hp_left);
        out.bool(w.all_spawned);
        out.u64(w.last_spawn_tick.unwrap_or(u64::MAX));
    }
    if let Some(l) = world.resources.get::<LaneState>() {
        for (h, a) in l.hose_available.iter().zip(l.active) {
            out.bool(*h);
            out.bool(a);
        }
    }
}

fn write_entity(world: &World, e: Entity, out: &mut Out) {
    out.entity(e);
    let get = |w: &World| w.get::<Transform2D>(e).map(|t| *t);
    out.opt(1, get(world), |o, t| {
        o.f32(t.pos.x);
        o.f32(t.pos.y);
    });
    out.opt(2, world.get::<Health>(e).map(|v| *v), |o, h| {
        o.i32(h.hp);
        o.i32(h.max);
    });
    out.opt(3, world.get::<DefenderRef>(e).map(|v| *v), |o, d| {
        o.str(d.id.as_str());
    });
    out.opt(4, world.get::<EnemyRef>(e).map(|v| *v), |o, d| {
        o.str(d.id.as_str());
    });
    out.opt(5, world.get::<Cell>(e).map(|v| *v), |o, c| {
        o.u8(c.row);
        o.u8(c.col);
    });
    out.opt(6, world.get::<Armor>(e).map(|v| *v), |o, a| o.i32(a.hp));
    out.opt(7, world.get::<StatusEffects>(e).map(|v| *v), |o, s| {
        o.u32(s.chill_ticks);
        o.u32(s.stun_ticks);
    });
    out.opt(8, world.get::<Walker>(e).map(|v| *v), |o, w| {
        o.f32(w.base_speed);
        match w.state {
            WalkState::Walking => o.u8(0),
            WalkState::Eating(t) => {
                o.u8(1);
                o.entity(t);
            }
            WalkState::Vaulting { from, to, t } => {
                o.u8(2);
                o.f32(from);
                o.f32(to);
                o.u32(t);
            }
            WalkState::Swallowed => o.u8(3),
        }
    });
    out.opt(9, world.get::<Biter>(e).map(|v| *v), |o, b| {
        o.u32(b.elapsed)
    });
    out.opt(10, world.get::<Shooter>(e).map(|v| *v), |o, s| {
        o.timer(s.timer);
        o.u8(s.volley_left);
        o.timer(s.volley_timer);
    });
    out.opt(11, world.get::<Producer>(e).map(|v| *v), |o, p| {
        o.timer(p.timer);
        o.u32(p.produced);
    });
    out.opt(12, world.get::<Spikes>(e).map(|v| *v), |o, s| {
        o.timer(s.timer)
    });
    out.opt(13, world.get::<Mine>(e).map(|v| *v), |o, m| {
        o.timer(m.arm);
        o.bool(m.armed);
    });
    out.opt(14, world.get::<Blast>(e).map(|v| *v), |o, b| {
        o.timer(b.fuse)
    });
    out.opt(15, world.get::<Chomper>(e).map(|v| *v), |o, c| {
        o.bool(c.state == ChompState::Ready);
        o.timer(c.chew);
    });
    out.opt(16, world.get::<Gust>(e).map(|v| *v), |o, g| o.timer(g.fuse));
    out.opt(17, world.get::<Vaulter>(e).map(|v| *v), |o, v| {
        o.bool(v.used)
    });
    out.opt(18, world.get::<Sprinter>(e).map(|v| *v), |o, s| {
        o.bool(s.triggered);
    });
    out.opt(19, world.get::<OverlayOf>(e).map(|v| v.host), Out::entity);
    out.opt(20, world.get::<Shielded>(e).map(|v| v.overlay), Out::entity);
    out.opt(
        21,
        world
            .get::<ProjectileBody>(e)
            .map(|b| (b.damage, b.origin_x, b.hit.len())),
        |o, (d, x, n)| {
            o.i32(d);
            o.f32(x);
            o.u32(u32::try_from(n).unwrap_or(u32::MAX));
        },
    );
    out.opt(22, world.get::<DewPickup>(e).map(|v| *v), |o, d| {
        o.i32(d.amount);
        o.f32(d.land_y);
        o.timer(d.expire);
        o.bool(d.landed);
    });
    out.u8(0xFF);
}
