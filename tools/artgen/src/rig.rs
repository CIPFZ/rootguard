//! 角色描述（ART-SPEC 9.1）：`rig.ron`（部件层级、枢轴、默认位姿、变体、受损状态）与
//! `clips.ron`（关键帧、变体切换、事件帧、时长绑定），以及在任一 tick 求各部件位姿。

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::palette::Material;

/// 角色类别：决定画布、锚点、材质、所属图集与尺寸上限（ART-SPEC 1.4 / 2.1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Kind {
    /// 守卫（站立）。
    Guardian,
    /// 守卫（贴地：荆棘垫、块茎雷）。
    GuardianGround,
    /// 葫芦壳（与宿主同锚）。
    Overlay,
    /// Husk（标准）。
    Husk,
    /// 鼓腹枯壳。
    HuskLarge,
    /// 小芽怪。
    HuskSmall,
    /// 弹丸。
    Projectile,
    /// 露水。
    Dew,
}

impl Kind {
    /// 画布像素（ART-SPEC 2.1）。
    #[must_use]
    pub fn canvas(self) -> (u32, u32) {
        match self {
            Self::Guardian | Self::Overlay => (128, 128),
            Self::GuardianGround => (128, 64),
            Self::Husk => (128, 160),
            Self::HuskLarge => (176, 176),
            Self::HuskSmall => (96, 96),
            Self::Projectile => (32, 32),
            Self::Dew => (48, 48),
        }
    }

    /// 归一化锚点（ART-SPEC 2.1）。
    #[must_use]
    pub fn anchor(self) -> (f32, f32) {
        match self {
            Self::Guardian | Self::Overlay => (0.5, 0.94),
            Self::GuardianGround => (0.5, 0.80),
            Self::Husk | Self::HuskLarge | Self::HuskSmall => (0.5, 0.95),
            Self::Projectile | Self::Dew => (0.5, 0.5),
        }
    }

    /// 材质（ART-SPEC 1.2）。
    #[must_use]
    pub fn material(self) -> Material {
        match self {
            Self::Guardian | Self::GuardianGround | Self::Overlay => Material::Guardian,
            Self::Husk | Self::HuskLarge | Self::HuskSmall => Material::Husk,
            Self::Projectile | Self::Dew => Material::Prop,
        }
    }

    /// 输出图集名（ART-SPEC 2.1）。
    #[must_use]
    pub fn atlas(self) -> &'static str {
        match self {
            Self::Guardian | Self::GuardianGround | Self::Overlay => "garden_defenders",
            Self::Husk | Self::HuskLarge | Self::HuskSmall => "garden_husks",
            Self::Projectile | Self::Dew => "garden_fx",
        }
    }

    /// 可见像素外接框的尺寸上限（宽, 高）（ART-SPEC 1.4）；无规定时为画布。
    #[must_use]
    pub fn size_limit(self) -> (u32, u32) {
        match self {
            Self::Guardian | Self::Overlay => (92, 104),
            Self::GuardianGround => (92, 48),
            Self::Husk => (128, 130),
            Self::HuskLarge => (176, 150),
            Self::HuskSmall => (96, 60),
            Self::Projectile | Self::Dew => self.canvas(),
        }
    }
}

/// 部件定义。
#[derive(Clone, Debug, Deserialize)]
pub struct PartDef {
    /// 部件名，对应 `parts/<name>.svg`；变体为 `parts/<name>_<variant>.svg`。
    pub name: String,
    /// 父部件：子部件叠加父部件的变换（如脸跟随身体）。
    #[serde(default)]
    pub parent: Option<String>,
    /// 旋转与缩放的枢轴（画布坐标）。
    pub pivot: (f32, f32),
    /// 绘制顺序（小者先画）。
    pub z: i32,
    /// 可切换的变体名；特殊值 `hidden` 表示隐藏，`base` 表示基础文件。
    #[serde(default)]
    pub variants: Vec<String>,
    /// 默认隐藏（只在切换到某变体时出现）。
    #[serde(default)]
    pub hidden: bool,
}

/// 受损 / 护甲状态（ART-SPEC 2.4）：以替换部件实现，输出片段名前缀 `<id>_<state>_<clip>`。
#[derive(Clone, Debug, Deserialize)]
pub struct StateDef {
    /// 状态名（片段名前缀）；空串为基础状态。
    pub name: String,
    /// 部件 → 变体。
    #[serde(default)]
    pub set: BTreeMap<String, String>,
}

/// `rig.ron`。
#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "Rig")]
pub struct Rig {
    /// 类别。
    pub kind: Kind,
    /// 部件。
    pub parts: Vec<PartDef>,
    /// 受损 / 护甲状态；为空时只有基础状态。
    #[serde(default)]
    pub states: Vec<StateDef>,
    /// 是否为每帧生成受伤闪白帧 `<帧名>_flash`（ART-SPEC 2.3）。
    #[serde(default)]
    pub flash: bool,
    /// 默认位姿取自哪个片段的第 0 帧（审核表、卡牌与尺寸校验用）。
    pub rest: String,
}

/// 片段时长。
#[derive(Clone, Debug, Deserialize)]
pub enum Duration {
    /// 固定 tick 数。
    Ticks(u32),
    /// 绑定规则数据（ART-SPEC 9.2），如 `Rule("enemies.husk.bite_interval_ticks")`。
    Rule(String),
}

/// 帧划分。
#[derive(Clone, Debug, Deserialize)]
pub enum Frames {
    /// 每帧 n tick，最后一帧吸收余数。
    Every(u32),
    /// 逐帧 tick 数，总和必须等于片段时长。
    List(Vec<u32>),
}

/// 缓动：本关键帧到下一关键帧之间的插值方式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Ease {
    /// 线性。
    Linear,
    /// 平滑（smoothstep）。
    #[default]
    Smooth,
    /// 保持到下一关键帧。
    Step,
}

const fn one() -> f32 {
    1.0
}

/// 关键帧：某 tick 的部件局部变换（相对默认位姿）。
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Key {
    /// tick。
    pub t: u32,
    /// 平移 x（px）。
    #[serde(default)]
    pub dx: f32,
    /// 平移 y（px）。
    #[serde(default)]
    pub dy: f32,
    /// 绕枢轴旋转（度，顺时针）。
    #[serde(default)]
    pub rot: f32,
    /// 绕枢轴缩放 x。
    #[serde(default = "one")]
    pub sx: f32,
    /// 绕枢轴缩放 y。
    #[serde(default = "one")]
    pub sy: f32,
    /// 到下一关键帧的缓动。
    #[serde(default)]
    pub ease: Ease,
}

/// 片段定义。
#[derive(Clone, Debug, Deserialize)]
pub struct ClipDef {
    /// 片段名（输出为 `<id>_<state>_<name>`）。
    pub name: String,
    /// 是否循环。
    pub looping: bool,
    /// 时长。
    pub duration: Duration,
    /// 帧划分。
    pub frames: Frames,
    /// 部件 → 关键帧。
    #[serde(default)]
    pub keys: BTreeMap<String, Vec<Key>>,
    /// 部件 → (tick, 变体) 阶跃切换。
    #[serde(default)]
    pub swaps: BTreeMap<String, Vec<(u32, String)>>,
    /// 事件帧：(tick, 事件名)；tick 必须是某帧的起点。
    #[serde(default)]
    pub events: Vec<(u32, String)>,
}

/// `clips.ron`。
#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "Clips")]
pub struct Clips(pub Vec<ClipDef>);

/// 部件局部变换。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local {
    /// 平移。
    pub dx: f32,
    /// 平移。
    pub dy: f32,
    /// 旋转（度）。
    pub rot: f32,
    /// 缩放。
    pub sx: f32,
    /// 缩放。
    pub sy: f32,
}

impl Local {
    /// 恒等变换。
    pub const IDENTITY: Self = Self {
        dx: 0.0,
        dy: 0.0,
        rot: 0.0,
        sx: 1.0,
        sy: 1.0,
    };

    fn of(k: &Key) -> Self {
        Self {
            dx: k.dx,
            dy: k.dy,
            rot: k.rot,
            sx: k.sx,
            sy: k.sy,
        }
    }

    fn lerp(a: Self, b: Self, t: f32) -> Self {
        let m = |x: f32, y: f32| x + (y - x) * t;
        Self {
            dx: m(a.dx, b.dx),
            dy: m(a.dy, b.dy),
            rot: m(a.rot, b.rot),
            sx: m(a.sx, b.sx),
            sy: m(a.sy, b.sy),
        }
    }
}

/// 2D 仿射矩阵 `[a b c d e f]`（SVG `matrix()` 顺序）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine(pub [f32; 6]);

impl Affine {
    /// 恒等。
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// `self × other`（先应用 `other`）。
    #[must_use]
    pub fn then(self, other: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = other.0;
        Self([
            a * a2 + c * b2,
            b * a2 + d * b2,
            a * c2 + c * d2,
            b * c2 + d * d2,
            a * e2 + c * f2 + e,
            b * e2 + d * f2 + f,
        ])
    }

    /// 局部变换：绕 `pivot` 缩放、旋转，再平移。
    #[must_use]
    pub fn local(l: Local, pivot: (f32, f32)) -> Self {
        let (px, py) = pivot;
        let r = l.rot.to_radians();
        let (s, c) = r.sin_cos();
        let to = Self([1.0, 0.0, 0.0, 1.0, px + l.dx, py + l.dy]);
        let rot = Self([c, s, -s, c, 0.0, 0.0]);
        let scale = Self([l.sx, 0.0, 0.0, l.sy, 0.0, 0.0]);
        let back = Self([1.0, 0.0, 0.0, 1.0, -px, -py]);
        to.then(rot).then(scale).then(back)
    }
}

/// 关键帧序列在 tick `t` 的局部变换。
#[must_use]
pub fn sample_keys(keys: &[Key], t: u32) -> Local {
    let Some(first) = keys.first() else {
        return Local::IDENTITY;
    };
    if t <= first.t {
        return Local::of(first);
    }
    for w in keys.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if t < b.t {
            let span = (b.t - a.t).max(1) as f32;
            let x = (t - a.t) as f32 / span;
            let x = match a.ease {
                Ease::Linear => x,
                Ease::Smooth => x * x * (3.0 - 2.0 * x),
                Ease::Step => 0.0,
            };
            return Local::lerp(Local::of(a), Local::of(b), x);
        }
    }
    keys.last().map_or(Local::IDENTITY, Local::of)
}

/// 阶跃切换在 tick `t` 的变体（无切换时为 `None`）。
#[must_use]
pub fn sample_swaps(swaps: &[(u32, String)], t: u32) -> Option<&str> {
    swaps
        .iter()
        .take_while(|(at, _)| *at <= t)
        .last()
        .map(|(_, v)| v.as_str())
}

/// 按帧划分得到各帧 tick 数。
///
/// # Errors
///
/// 帧时长为 0、或 `List` 之和不等于 `duration` 时返回错误。
pub fn frame_ticks(frames: &Frames, duration: u32) -> Result<Vec<u32>> {
    let ticks = match frames {
        Frames::Every(n) => {
            if *n == 0 {
                bail!("Every(0) 无效");
            }
            let count = (duration / n).max(1);
            let mut v = vec![*n; count as usize];
            if let Some(last) = v.last_mut() {
                *last += duration - n * count;
            }
            v
        }
        Frames::List(v) => v.clone(),
    };
    if ticks.contains(&0) {
        bail!("帧时长不能为 0");
    }
    let sum: u32 = ticks.iter().sum();
    if sum != duration {
        bail!("帧时长之和 {sum} ≠ 片段时长 {duration}");
    }
    Ok(ticks)
}

/// 一帧中各部件的绘制指令：(部件, 变体, 世界变换)，按 z 排序，已去掉隐藏部件。
pub type Pose<'r> = Vec<(&'r PartDef, Option<&'r str>, Affine)>;

impl Rig {
    /// 校验部件层级（父部件存在、无环、名称唯一）与状态引用的部件 / 变体。
    ///
    /// # Errors
    ///
    /// 结构不一致时返回错误。
    pub fn check(&self) -> Result<()> {
        let mut names = std::collections::BTreeSet::new();
        for p in &self.parts {
            if !names.insert(p.name.as_str()) {
                bail!("部件名重复: {}", p.name);
            }
        }
        for p in &self.parts {
            let mut seen = 0;
            let mut cur = p.parent.as_deref();
            while let Some(name) = cur {
                seen += 1;
                if seen > self.parts.len() {
                    bail!("部件 {} 的父链成环", p.name);
                }
                cur = self
                    .part(name)
                    .with_context(|| format!("部件 {} 的父部件 {name} 不存在", p.name))?
                    .parent
                    .as_deref();
            }
        }
        for s in &self.states {
            for (part, variant) in &s.set {
                self.check_variant(part, variant)
                    .with_context(|| format!("状态 {:?}", s.name))?;
            }
        }
        Ok(())
    }

    /// 部件 `part` 是否有变体 `variant`（含特殊值 `base` / `hidden`）。
    ///
    /// # Errors
    ///
    /// 部件或变体不存在时返回错误。
    pub fn check_variant(&self, part: &str, variant: &str) -> Result<()> {
        let p = self
            .part(part)
            .with_context(|| format!("部件 {part} 不存在"))?;
        if variant == "base" || variant == "hidden" || p.variants.iter().any(|v| v == variant) {
            Ok(())
        } else {
            bail!("部件 {part} 没有变体 {variant}")
        }
    }

    /// 按名查找部件。
    #[must_use]
    pub fn part(&self, name: &str) -> Option<&PartDef> {
        self.parts.iter().find(|p| p.name == name)
    }

    /// 输出状态列表（未声明时只有基础状态）。
    #[must_use]
    pub fn output_states(&self) -> Vec<StateDef> {
        if self.states.is_empty() {
            vec![StateDef {
                name: String::new(),
                set: BTreeMap::new(),
            }]
        } else {
            self.states.clone()
        }
    }

    /// 片段 `clip` 在 tick `t`、状态 `state` 下的位姿。状态的变体覆盖片段切换（受损是持续状态）。
    #[must_use]
    pub fn pose<'r>(&'r self, clip: &'r ClipDef, state: &'r StateDef, t: u32) -> Pose<'r> {
        let mut world: BTreeMap<&str, Affine> = BTreeMap::new();
        // 父部件先求：按层级深度处理
        let depth = |p: &PartDef| {
            let mut d = 0;
            let mut cur = p.parent.as_deref();
            while let Some(n) = cur {
                d += 1;
                cur = self.part(n).and_then(|q| q.parent.as_deref());
            }
            d
        };
        let mut order: Vec<&PartDef> = self.parts.iter().collect();
        order.sort_by_key(|p| depth(p));
        for p in order {
            let local = clip
                .keys
                .get(&p.name)
                .map_or(Local::IDENTITY, |k| sample_keys(k, t));
            let parent = p
                .parent
                .as_deref()
                .and_then(|n| world.get(n).copied())
                .unwrap_or(Affine::IDENTITY);
            world.insert(&p.name, parent.then(Affine::local(local, p.pivot)));
        }
        let mut out: Pose<'r> = Vec::new();
        for p in &self.parts {
            let variant = state
                .set
                .get(&p.name)
                .map(String::as_str)
                .or_else(|| clip.swaps.get(&p.name).and_then(|s| sample_swaps(s, t)));
            let hidden = match variant {
                Some("hidden") => true,
                Some("base") => false,
                Some(_) => false,
                None => p.hidden,
            };
            if hidden {
                continue;
            }
            let variant = variant.filter(|v| *v != "base");
            out.push((
                p,
                variant,
                world
                    .get(p.name.as_str())
                    .copied()
                    .unwrap_or(Affine::IDENTITY),
            ));
        }
        out.sort_by_key(|(p, _, _)| p.z);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(t: u32, dy: f32, ease: Ease) -> Key {
        Key {
            t,
            dx: 0.0,
            dy,
            rot: 0.0,
            sx: 1.0,
            sy: 1.0,
            ease,
        }
    }

    #[test]
    fn keys_interpolate_and_clamp() {
        let k = [
            key(10, 0.0, Ease::Linear),
            key(20, 10.0, Ease::Step),
            key(30, 0.0, Ease::Linear),
        ];
        assert_eq!(sample_keys(&k, 0).dy, 0.0);
        assert_eq!(sample_keys(&k, 15).dy, 5.0);
        assert_eq!(sample_keys(&k, 25).dy, 10.0, "Step 保持");
        assert_eq!(sample_keys(&k, 99).dy, 0.0);
        let smooth = [key(0, 0.0, Ease::Smooth), key(10, 10.0, Ease::Linear)];
        assert!(sample_keys(&smooth, 2).dy < 2.0, "smoothstep 起步慢");
    }

    #[test]
    fn frame_ticks_sum_to_duration() {
        assert_eq!(
            frame_ticks(&Frames::Every(8), 30).ok(),
            Some(vec![8, 8, 14])
        );
        assert_eq!(
            frame_ticks(&Frames::List(vec![30, 20, 10]), 60).ok(),
            Some(vec![30, 20, 10])
        );
        assert!(frame_ticks(&Frames::List(vec![30, 20]), 60).is_err());
        assert!(frame_ticks(&Frames::List(vec![60, 0]), 60).is_err());
    }

    #[test]
    fn affine_rotates_around_pivot() {
        let m = Affine::local(
            Local {
                rot: 90.0,
                ..Local::IDENTITY
            },
            (10.0, 10.0),
        );
        let [a, b, c, d, e, f] = m.0;
        let (x, y) = (20.0, 10.0);
        let (tx, ty) = (a * x + c * y + e, b * x + d * y + f);
        assert!(
            (tx - 10.0).abs() < 1e-4 && (ty - 20.0).abs() < 1e-4,
            "({tx}, {ty})"
        );
    }

    #[test]
    fn swaps_step_and_state_overrides() {
        assert_eq!(
            sample_swaps(&[(0, "a".into()), (5, "b".into())], 4),
            Some("a")
        );
        assert_eq!(sample_swaps(&[(3, "b".into())], 2), None);
    }
}
