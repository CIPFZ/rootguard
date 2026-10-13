//! 文本表（SPEC 11.6）：`assets/data/i18n/{zh,en}.ron`，代码通过 [`t`] 取用。
//!
//! 缺失 key 显示 key 本身并 `log::warn!` 一次。中文显示依赖引擎补充 E-1（CJK 字体）；
//! 在游戏所用的引擎 rev 支持之前，界面以英文显示，中文表仍保持齐备（SPEC 11.6，ADR-004）。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use rootguard_core::events::Lang;
use rootguard_core::resources::Progress;
use trellia::prelude::*;

/// 当前引擎 rev 是否能显示 CJK 字形（E-1 合入并 bump rev 后改为 true，ADR-004）。
pub const CJK_FONT_AVAILABLE: bool = false;

/// 文本表资源。
pub struct I18n {
    zh: BTreeMap<String, String>,
    en: BTreeMap<String, String>,
    warned: Mutex<BTreeSet<String>>,
}

impl I18n {
    /// 由两张表构造。
    #[must_use]
    pub fn new(zh: BTreeMap<String, String>, en: BTreeMap<String, String>) -> Self {
        Self {
            zh,
            en,
            warned: Mutex::new(BTreeSet::new()),
        }
    }

    /// 从资产目录加载两张表。
    ///
    /// # Errors
    ///
    /// 文件缺失或 RON 解析失败时返回 [`AssetError`]（含路径）。
    pub fn load(server: &AssetServer) -> Result<Self, AssetError> {
        Ok(Self::new(
            server.load_ron("data/i18n/zh.ron")?,
            server.load_ron("data/i18n/en.ron")?,
        ))
    }

    /// 实际显示语言：设置为中文但字体不可用时回退英文。
    #[must_use]
    pub fn display_lang(lang: Lang) -> Lang {
        if lang == Lang::Zh && !CJK_FONT_AVAILABLE {
            Lang::En
        } else {
            lang
        }
    }

    /// 取文本；缺失时返回 key 本身并警告一次。
    #[must_use]
    pub fn get<'a>(&'a self, lang: Lang, key: &'a str) -> &'a str {
        let table = match lang {
            Lang::Zh => &self.zh,
            Lang::En => &self.en,
        };
        if let Some(v) = table.get(key) {
            return v;
        }
        if let Ok(mut warned) = self.warned.lock()
            && warned.insert(key.to_owned())
        {
            log::warn!("i18n: 缺少 key `{key}`（{lang:?}）");
        }
        key
    }

    /// 两张表的 key 集合差异：(只在中文表, 只在英文表)。
    #[must_use]
    pub fn key_diff(&self) -> (Vec<&str>, Vec<&str>) {
        let only_zh = self
            .zh
            .keys()
            .filter(|k| !self.en.contains_key(*k))
            .map(String::as_str)
            .collect();
        let only_en = self
            .en
            .keys()
            .filter(|k| !self.zh.contains_key(*k))
            .map(String::as_str)
            .collect();
        (only_zh, only_en)
    }

    /// 是否含有某 key（两张表任一）。
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.zh.contains_key(key) && self.en.contains_key(key)
    }
}

/// 当前语言下的文本（SPEC 11.6 `t("key")`）。无文本表时返回 key。
#[must_use]
pub fn t(world: &World, key: &str) -> String {
    let lang = world
        .resources
        .get::<Progress>()
        .map_or(Lang::default(), |p| p.settings.lang);
    match world.resources.get::<I18n>() {
        Some(i18n) => i18n.get(I18n::display_lang(lang), key).to_owned(),
        None => key.to_owned(),
    }
}

/// 带占位符替换的文本：`{name}` → 值。
#[must_use]
pub fn tf(world: &World, key: &str, args: &[(&str, &str)]) -> String {
    let mut s = t(world, key);
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_falls_back_to_key() {
        let mut en = BTreeMap::new();
        en.insert("a".to_owned(), "A".to_owned());
        let i18n = I18n::new(BTreeMap::new(), en);
        assert_eq!(i18n.get(Lang::En, "a"), "A");
        assert_eq!(i18n.get(Lang::Zh, "a"), "a");
        assert_eq!(i18n.get(Lang::Zh, "a"), "a");
        assert_eq!(i18n.key_diff(), (vec![], vec!["a"]));
    }
}
