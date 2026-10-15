//! 时长绑定（ART-SPEC 3.1 / 9.2）：`clips.ron` 中 `Rule("…")` 从 `assets/data/**/*.ron` 读取数值，
//! 以及“哪些片段必须绑定哪条规则”的约定。数据经 `rootguard-content` 加载并校验，与游戏读取的完全相同。

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use rootguard_core::Content;
use rootguard_core::data::Behavior;

/// 已加载的规则数据。
pub struct Rules {
    content: Content,
}

impl Rules {
    /// 从资产根目录（`assets/`）加载。
    ///
    /// # Errors
    ///
    /// 数据加载或校验失败时返回错误。
    pub fn load(assets: &Path) -> Result<Self> {
        let content = rootguard_content::load_from(assets)
            .map_err(|e| anyhow!("{e}"))
            .context("加载 assets/data")?;
        Ok(Self { content })
    }

    /// 解析规则路径：
    ///
    /// - `defenders.<id>.<behavior>.<field>`，behavior 为 `shooter` / `producer` / `spikes` / `mine` /
    ///   `blast` / `chomper` / `gust`，field 为该行为的 tick 或计数字段（SPEC 4.1）；
    /// - `enemies.<id>.bite_interval_ticks`（SPEC 4.3）；
    /// - `enemies.<id>.vault.ticks`（Vault 特性的跨越耗时，ADR-002）。
    ///
    /// # Errors
    ///
    /// 路径格式错误、id 不存在、行为不存在或字段不是整数时返回错误。
    pub fn value(&self, path: &str) -> Result<u32> {
        let parts: Vec<&str> = path.split('.').collect();
        match parts.as_slice() {
            ["defenders", id, behavior, field] => {
                let def = self
                    .content
                    .defenders
                    .iter()
                    .find(|(k, _)| k.as_str() == *id)
                    .map(|(_, d)| d)
                    .with_context(|| format!("规则 {path}：没有防御者 {id}"))?;
                def.behaviors
                    .iter()
                    .find_map(|b| behavior_field(b, behavior, field))
                    .with_context(|| format!("规则 {path}：{id} 没有 {behavior}.{field}"))
            }
            ["enemies", id, "vault", "ticks"] => self
                .content
                .enemies
                .iter()
                .find(|(k, _)| k.as_str() == *id)
                .and_then(|(_, e)| {
                    e.traits.iter().find_map(|t| match t {
                        rootguard_core::data::Trait::Vault { ticks } => Some(*ticks),
                        _ => None,
                    })
                })
                .with_context(|| format!("规则 {path}：{id} 没有 Vault 特性")),
            ["enemies", id, "bite_interval_ticks"] => self
                .content
                .enemies
                .iter()
                .find(|(k, _)| k.as_str() == *id)
                .map(|(_, e)| e.bite_interval_ticks)
                .with_context(|| format!("规则 {path}：没有敌人 {id}")),
            _ => bail!("规则路径 {path:?} 无法识别（见 artgen::rules::Rules::value）"),
        }
    }

    /// 片段 `clip` 必须绑定的规则路径（ART-SPEC 3.1“驱动方式”中由计时器决定时长的片段）；
    /// 不受约束的片段返回 `None`。
    #[must_use]
    pub fn required_binding(&self, id: &str, clip: &str) -> Option<String> {
        if let Some((_, def)) = self
            .content
            .defenders
            .iter()
            .find(|(k, _)| k.as_str() == id)
        {
            let key = match clip {
                "attack" => "shooter.interval_ticks",
                "produce" => "producer.interval_ticks",
                "arm" => "mine.arm_ticks",
                "trigger" => "spikes.interval_ticks",
                "fuse"
                    if def
                        .behaviors
                        .iter()
                        .any(|b| matches!(b, Behavior::Gust { .. })) =>
                {
                    "gust.fuse_ticks"
                }
                "fuse" => "blast.fuse_ticks",
                _ => return None,
            };
            return Some(format!("defenders.{id}.{key}"));
        }
        match clip {
            "eat" => Some(format!("enemies.{id}.bite_interval_ticks")),
            // 跨越时长 = Vault.ticks（SPEC 8.1 / ADR-002）；ART-SPEC 5.5 写 30，以数据为准（ADR-016 第 4 节）
            "vault" => Some(format!("enemies.{id}.vault.ticks")),
            _ => None,
        }
    }

    /// 射手每次连发的弹丸数（`attack` 片段的 `fire` 事件数，ART-SPEC 9.2）。
    #[must_use]
    pub fn volley(&self, id: &str) -> Option<u32> {
        self.value(&format!("defenders.{id}.shooter.volley")).ok()
    }
}

fn behavior_field(b: &Behavior, behavior: &str, field: &str) -> Option<u32> {
    match (b, behavior, field) {
        (Behavior::Shooter { interval_ticks, .. }, "shooter", "interval_ticks")
        | (Behavior::Producer { interval_ticks, .. }, "producer", "interval_ticks")
        | (Behavior::Spikes { interval_ticks, .. }, "spikes", "interval_ticks") => {
            Some(*interval_ticks)
        }
        (Behavior::Shooter { volley, .. }, "shooter", "volley") => Some(u32::from(*volley)),
        (
            Behavior::Shooter {
                volley_gap_ticks, ..
            },
            "shooter",
            "volley_gap_ticks",
        ) => Some(*volley_gap_ticks),
        (Behavior::Mine { arm_ticks, .. }, "mine", "arm_ticks") => Some(*arm_ticks),
        (Behavior::Blast { fuse_ticks, .. }, "blast", "fuse_ticks")
        | (Behavior::Gust { fuse_ticks, .. }, "gust", "fuse_ticks") => Some(*fuse_ticks),
        (Behavior::Chomper { chew_ticks, .. }, "chomper", "chew_ticks") => Some(*chew_ticks),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Rules {
        Rules::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")).expect("规则")
    }

    #[test]
    fn resolves_rule_paths_from_data() {
        let r = rules();
        assert_eq!(
            r.value("defenders.pebble_pod.shooter.interval_ticks").ok(),
            Some(85)
        );
        assert_eq!(r.value("defenders.twinpod.shooter.volley").ok(), Some(2));
        assert_eq!(
            r.value("defenders.tuberblast.mine.arm_ticks").ok(),
            Some(900)
        );
        assert_eq!(r.value("enemies.husk.bite_interval_ticks").ok(), Some(60));
        assert!(r.value("defenders.pebble_pod.mine.arm_ticks").is_err());
        assert!(r.value("defenders.nobody.shooter.interval_ticks").is_err());
        assert!(r.value("enemies.husk.speed").is_err());
    }

    #[test]
    fn required_bindings_follow_art_spec_3_1() {
        let r = rules();
        assert_eq!(
            r.required_binding("pebble_pod", "attack").as_deref(),
            Some("defenders.pebble_pod.shooter.interval_ticks")
        );
        assert_eq!(
            r.required_binding("gustbloom", "fuse").as_deref(),
            Some("defenders.gustbloom.gust.fuse_ticks")
        );
        assert_eq!(
            r.required_binding("emberbulb", "fuse").as_deref(),
            Some("defenders.emberbulb.blast.fuse_ticks")
        );
        assert_eq!(
            r.required_binding("husk", "eat").as_deref(),
            Some("enemies.husk.bite_interval_ticks")
        );
        assert_eq!(r.required_binding("husk", "walk"), None);
        assert_eq!(r.required_binding("pebble_pod", "idle"), None);
    }
}
