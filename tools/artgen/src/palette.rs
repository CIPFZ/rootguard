//! 调色板（ART-SPEC 1.3）：`art/palette.ron` 的全局色 + 每角色 ≤ 2 个专属色。
//!
//! SVG 部件只能以 `@名` 引用颜色，`@名.shade` 为一阶赛璐璐阴影（ART-SPEC 1.2：守卫主色降 18% 明度、
//! Husk 降 22%）。合成时替换为十六进制值；渲染前另以 usvg 树逐个检查实际颜色（[`Palette::allowed`]）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// 每个角色最多的专属色数（ART-SPEC 1.3）。
pub const MAX_CHARACTER_COLORS: usize = 2;

/// sRGB 颜色。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// 解析 `#RRGGBB`。
    ///
    /// # Errors
    ///
    /// 格式不是 `#` 加 6 位十六进制时返回错误。
    pub fn parse(s: &str) -> Result<Self> {
        let hex = s
            .strip_prefix('#')
            .filter(|h| h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit()))
            .with_context(|| format!("颜色 {s:?} 不是 #RRGGBB"))?;
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or_default();
        Ok(Self(byte(0), byte(2), byte(4)))
    }

    /// `#RRGGBB` 形式。
    #[must_use]
    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// 明度（HSV 的 V）降低 `amount`（0..1），色相与饱和度不变。
    #[must_use]
    pub fn darken(self, amount: f64) -> Self {
        let k = (1.0 - amount).clamp(0.0, 1.0);
        // 结果已钳制到 0..=255，转换无截断
        let ch = |c: u8| (f64::from(c) * k).round().clamp(0.0, 255.0) as u8;
        Self(ch(self.0), ch(self.1), ch(self.2))
    }
}

/// 材质（ART-SPEC 1.2）：决定阴影降明度的比例与描边色。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Material {
    /// 守卫（温室植物）。
    Guardian,
    /// Husk（干枯壳）。
    Husk,
    /// 弹丸、露水、特效等无材质对立的物件（阴影同守卫）。
    Prop,
}

#[derive(Deserialize)]
struct ShadeDef {
    guardian: f64,
    husk: f64,
}

#[derive(Deserialize)]
#[serde(rename = "Palette")]
struct PaletteDef {
    global: BTreeMap<String, String>,
    shade: ShadeDef,
    characters: BTreeMap<String, BTreeMap<String, String>>,
}

/// 已解析的调色板。
#[derive(Clone, Debug)]
pub struct Palette {
    global: BTreeMap<String, Rgb>,
    characters: BTreeMap<String, BTreeMap<String, Rgb>>,
    shade_guardian: f64,
    shade_husk: f64,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl Palette {
    /// 读取并校验 `art/palette.ron`。
    ///
    /// # Errors
    ///
    /// 文件缺失、解析失败、颜色格式错误、名称非法、专属色超过 2 个或与全局色重名时返回错误。
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("读取调色板 {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("调色板 {}", path.display()))
    }

    /// 从 RON 文本解析（见 [`Palette::load`]）。
    ///
    /// # Errors
    ///
    /// 同 [`Palette::load`]。
    pub fn parse(text: &str) -> Result<Self> {
        let def: PaletteDef = ron::from_str(text)?;
        let parse_map = |map: &BTreeMap<String, String>| -> Result<BTreeMap<String, Rgb>> {
            map.iter()
                .map(|(k, v)| {
                    if !valid_name(k) {
                        bail!("颜色名 {k:?} 只能含 [a-z0-9_]");
                    }
                    Ok((
                        k.clone(),
                        Rgb::parse(v).with_context(|| format!("颜色 {k}"))?,
                    ))
                })
                .collect()
        };
        let global = parse_map(&def.global)?;
        let mut characters = BTreeMap::new();
        for (id, map) in &def.characters {
            let colors = parse_map(map).with_context(|| format!("角色 {id}"))?;
            if colors.len() > MAX_CHARACTER_COLORS {
                bail!(
                    "角色 {id} 有 {} 个专属色，上限 {MAX_CHARACTER_COLORS}（ART-SPEC 1.3）",
                    colors.len()
                );
            }
            if let Some(dup) = colors.keys().find(|k| global.contains_key(*k)) {
                bail!("角色 {id} 的专属色 {dup} 与全局色重名");
            }
            characters.insert(id.clone(), colors);
        }
        for (name, amount) in [("guardian", def.shade.guardian), ("husk", def.shade.husk)] {
            if !(0.0..1.0).contains(&amount) {
                bail!("shade.{name} = {amount} 应在 [0, 1) 内");
            }
        }
        Ok(Self {
            global,
            characters,
            shade_guardian: def.shade.guardian,
            shade_husk: def.shade.husk,
        })
    }

    fn shade_amount(&self, material: Material) -> f64 {
        match material {
            Material::Husk => self.shade_husk,
            Material::Guardian | Material::Prop => self.shade_guardian,
        }
    }

    /// 角色 `character` 可用的基础色（全局 + 专属）。
    fn base(&self, character: &str) -> impl Iterator<Item = (&str, Rgb)> {
        self.global
            .iter()
            .chain(self.characters.get(character).into_iter().flatten())
            .map(|(k, v)| (k.as_str(), *v))
    }

    /// 解析部件中的颜色引用 `name` 或 `name.shade`（不含 `@`）。
    #[must_use]
    pub fn resolve(&self, character: &str, material: Material, token: &str) -> Option<Rgb> {
        let (name, shade) = match token.strip_suffix(".shade") {
            Some(base) => (base, true),
            None => (token, false),
        };
        let color = self.base(character).find(|(k, _)| *k == name)?.1;
        Some(if shade {
            color.darken(self.shade_amount(material))
        } else {
            color
        })
    }

    /// 角色可出现的全部颜色（基础色及其阴影色），用于渲染前的颜色校验（ART-SPEC 9.2）。
    #[must_use]
    pub fn allowed(&self, character: &str, material: Material) -> BTreeSet<Rgb> {
        let amount = self.shade_amount(material);
        self.base(character)
            .flat_map(|(_, c)| [c, c.darken(amount)])
            .collect()
    }

    /// 某颜色在调色板中的名字（错误信息用）；不存在时为 `None`。
    #[must_use]
    pub fn name_of(&self, character: &str, material: Material, color: Rgb) -> Option<String> {
        let amount = self.shade_amount(material);
        self.base(character).find_map(|(k, c)| {
            if c == color {
                Some(k.to_owned())
            } else if c.darken(amount) == color {
                Some(format!("{k}.shade"))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = r##"Palette(
        global: { "leaf": "#5DBB63", "outline": "#2F3A2A" },
        shade: (guardian: 0.18, husk: 0.22),
        characters: { "pebble_pod": { "pebble": "#9A9A94" } },
    )"##;

    #[test]
    fn resolves_tokens_and_shades() {
        let p = Palette::parse(TEXT).expect("解析");
        let leaf = Rgb(0x5D, 0xBB, 0x63);
        assert_eq!(
            p.resolve("pebble_pod", Material::Guardian, "leaf"),
            Some(leaf)
        );
        assert_eq!(
            p.resolve("pebble_pod", Material::Guardian, "leaf.shade"),
            Some(leaf.darken(0.18))
        );
        assert_eq!(
            p.resolve("husk", Material::Husk, "leaf.shade"),
            Some(leaf.darken(0.22))
        );
        assert!(
            p.resolve("pebble_pod", Material::Guardian, "pebble")
                .is_some()
        );
        assert_eq!(
            p.resolve("husk", Material::Husk, "pebble"),
            None,
            "专属色只属于该角色"
        );
        assert_eq!(p.allowed("pebble_pod", Material::Guardian).len(), 6);
        assert_eq!(leaf.darken(0.18).hex(), "#4C9951");
    }

    #[test]
    fn rejects_bad_palettes() {
        let three = TEXT.replace(
            r##"{ "pebble": "#9A9A94" }"##,
            r##"{ "a": "#000001", "b": "#000002", "c": "#000003" }"##,
        );
        assert!(Palette::parse(&three).is_err(), "专属色超过 2 个");
        let dup = TEXT.replace(r#""pebble":"#, r#""leaf":"#);
        assert!(Palette::parse(&dup).is_err(), "与全局色重名");
        let bad = TEXT.replace("#5DBB63", "#5DBB6");
        assert!(Palette::parse(&bad).is_err(), "格式错误");
    }
}
