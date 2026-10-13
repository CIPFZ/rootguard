//! 只读注册表（SPEC 3.3 / 5.2）：由 `rootguard-content` 加载、校验后整体交给 [`ContentPlugin`](crate::ContentPlugin)。

use std::collections::BTreeMap;

use crate::data::{DefenderDef, EnemyDef, LevelDef, ProjectileDef, Rules};
use crate::ids::{DefenderId, EnemyId, LevelId, ProjectileId};

/// id → 定义的只读表，按 id 字典序迭代（确定性，SPEC 4.3 / 引擎 12）。
#[derive(Clone, Debug, PartialEq)]
pub struct Registry<K: Ord, V> {
    map: BTreeMap<K, V>,
}

impl<K: Ord, V> Default for Registry<K, V> {
    fn default() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
}

impl<K: Ord + Copy, V> Registry<K, V> {
    /// 插入定义，返回同 id 的旧定义（若有）。
    pub fn insert(&mut self, id: K, def: V) -> Option<V> {
        self.map.insert(id, def)
    }

    /// 按 id 查找。
    #[must_use]
    pub fn get(&self, id: K) -> Option<&V> {
        self.map.get(&id)
    }

    /// 是否存在。
    #[must_use]
    pub fn contains(&self, id: K) -> bool {
        self.map.contains_key(&id)
    }

    /// 按 id 字典序迭代。
    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
        self.map.iter().map(|(k, v)| (*k, v))
    }

    /// 条目数。
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// 防御者注册表（SPEC 5.2）。
pub type DefenderRegistry = Registry<DefenderId, DefenderDef>;
/// 敌人注册表（SPEC 5.2）。
pub type EnemyRegistry = Registry<EnemyId, EnemyDef>;
/// 弹丸注册表（SPEC 5.2）。
pub type ProjectileRegistry = Registry<ProjectileId, ProjectileDef>;

/// 关卡注册表（SPEC 5.2）：按冒险顺序（`LevelId::order_key`）排列。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LevelRegistry {
    levels: Vec<LevelDef>,
}

impl LevelRegistry {
    /// 由关卡列表构造，按冒险顺序排序。
    #[must_use]
    pub fn new(mut levels: Vec<LevelDef>) -> Self {
        levels.sort_by_key(|l| (l.id.order_key(), l.id));
        Self { levels }
    }

    /// 按 id 查找。
    #[must_use]
    pub fn get(&self, id: LevelId) -> Option<&LevelDef> {
        self.levels.iter().find(|l| l.id == id)
    }

    /// 冒险顺序中的下标。
    #[must_use]
    pub fn index_of(&self, id: LevelId) -> Option<usize> {
        self.levels.iter().position(|l| l.id == id)
    }

    /// 冒险顺序中的下一关。
    #[must_use]
    pub fn next(&self, id: LevelId) -> Option<&LevelDef> {
        self.index_of(id).and_then(|i| self.levels.get(i + 1))
    }

    /// 按冒险顺序迭代。
    pub fn iter(&self) -> impl Iterator<Item = &LevelDef> {
        self.levels.iter()
    }

    /// 关卡数。
    #[must_use]
    pub fn len(&self) -> usize {
        self.levels.len()
    }

    /// 是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }
}

/// 全部内置内容（已校验）。`rootguard-content` 产出，[`ContentPlugin`](crate::ContentPlugin) 插入为资源。
#[derive(Clone, Debug)]
pub struct Content {
    /// 防御者。
    pub defenders: DefenderRegistry,
    /// 弹丸。
    pub projectiles: ProjectileRegistry,
    /// 敌人。
    pub enemies: EnemyRegistry,
    /// 关卡。
    pub levels: LevelRegistry,
    /// 全局规则。
    pub rules: Rules,
}
