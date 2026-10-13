//! 场景导航与设置（SPEC 3.4 / 11.1 / 11.3）：唯一消费场景类 `UiAction` 的地方（ADR-004）。
//!
//! 场景只负责把按钮 / 按键翻译为 `UiAction`；本模块的 `ui_router_system`（`Update`，先于场景）
//! 根据当前会话执行切换、暂停与设置变更。

use rootguard_core::data::CardRule;
use rootguard_core::events::UiAction;
use rootguard_core::registry::LevelRegistry;
use rootguard_core::resources::Progress;
use rootguard_core::{DefenderId, LevelId};
use trellia::prelude::*;

use crate::save;
use crate::scenes::{
    BattleScene, LevelSelectScene, PauseScene, SeedSelectScene, TitleScene, take_scene_actions,
};
use crate::sfx;

/// 资源：当前会话（最近进入的关卡与所选卡牌），供重开 / 下一关使用。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Session {
    /// 关卡。
    pub level: Option<LevelId>,
    /// 所选卡牌（`Forced` 关为空）。
    pub cards: Vec<DefenderId>,
}

/// 关卡是否已解锁：冒险顺序第一关，或上一关已通关（SPEC 1.3 线性解锁）。
#[must_use]
pub fn level_unlocked(world: &World, id: LevelId) -> bool {
    let (Some(levels), Some(progress)) = (
        world.resources.get::<LevelRegistry>(),
        world.resources.get::<Progress>(),
    ) else {
        return false;
    };
    match levels.index_of(id) {
        Some(0) => true,
        Some(i) => levels
            .iter()
            .nth(i - 1)
            .is_some_and(|prev| progress.cleared.contains(&prev.id)),
        None => false,
    }
}

/// 清空场景栈并进入新场景（Title 为栈底，主场景之间 replace 语义，SPEC 3.4）。
fn goto(world: &mut World, scene: impl Scene) {
    let commands = world.resources.expect_mut::<SceneCommands>();
    commands.clear();
    commands.push(scene);
}

fn top(world: &World) -> Option<&'static str> {
    world
        .resources
        .get::<SceneStack>()
        .and_then(SceneStack::top_name)
}

/// 进入关卡：`Forced` 关直接对局，`Choose` 关先选卡（SPEC 11.3）。
fn start_level(world: &mut World, id: LevelId) {
    let Some(level) = world
        .resources
        .get::<LevelRegistry>()
        .and_then(|l| l.get(id))
        .cloned()
    else {
        log::warn!("StartLevel: 关卡不存在 {id}");
        return;
    };
    if !level_unlocked(world, id) {
        log::warn!("StartLevel: 关卡 {id} 尚未解锁");
        return;
    }
    let session = world.resources.expect_mut::<Session>();
    session.level = Some(id);
    match level.cards {
        CardRule::Forced(_) => {
            session.cards.clear();
            goto(world, BattleScene::new(id, Vec::new(), None));
        }
        CardRule::Choose => goto(world, SeedSelectScene::new(id)),
    }
}

fn update_settings(world: &mut World, f: impl FnOnce(&mut rootguard_core::resources::Settings)) {
    if let Some(p) = world.resources.get_mut::<Progress>() {
        f(&mut p.settings);
    }
    save::save(world);
}

/// `Update`：处理场景类 `UiAction`。
pub fn ui_router_system(world: &mut World) {
    for action in take_scene_actions(world) {
        match action {
            UiAction::StartLevel(id) => start_level(world, id),
            UiAction::ConfirmSeeds(cards) => {
                let Some(level) = world.resources.expect::<Session>().level else {
                    continue;
                };
                if cards.is_empty() {
                    continue;
                }
                world.resources.expect_mut::<Session>().cards = cards.clone();
                goto(world, BattleScene::new(level, cards, None));
            }
            UiAction::Restart => {
                let session = world.resources.expect::<Session>().clone();
                if let Some(level) = session.level {
                    goto(world, BattleScene::new(level, session.cards, None));
                }
            }
            UiAction::NextLevel => {
                let next = world.resources.expect::<Session>().level.and_then(|id| {
                    world
                        .resources
                        .get::<LevelRegistry>()
                        .and_then(|l| l.next(id))
                        .map(|l| l.id)
                });
                match next {
                    Some(id) => start_level(world, id),
                    None => goto(world, LevelSelectScene),
                }
            }
            UiAction::BackToTitle => goto(world, TitleScene),
            UiAction::BackToLevelSelect => goto(world, LevelSelectScene),
            UiAction::PauseToggle => match top(world) {
                Some("Battle") => world
                    .resources
                    .expect_mut::<SceneCommands>()
                    .push(PauseScene),
                Some("Paused") => world.resources.expect_mut::<SceneCommands>().pop(),
                _ => {}
            },
            UiAction::SetVolume { bus, volume } => {
                let volume = volume.clamp(0.0, 1.0);
                update_settings(world, |s| match bus {
                    rootguard_core::events::VolumeBus::Sfx => s.sfx = volume,
                    rootguard_core::events::VolumeBus::Music => s.music = volume,
                });
                if let Some(queue) = world.resources.get_mut::<AudioQueue>() {
                    queue.set_bus_volume(sfx::engine_bus(bus), volume);
                }
            }
            UiAction::SetLanguage(lang) => update_settings(world, |s| s.lang = lang),
            UiAction::SetSkin(skin) => update_settings(world, |s| s.skin = skin),
            // 对局动作由 buffer_ui_actions 移入收件箱，不会到达这里
            _ => {}
        }
    }
}

/// SPEC 3.3 `UiPlugin`：会话资源与导航系统。须在 `ScenePlugin` 之前添加，使导航先于场景 `update`。
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn name(&self) -> &'static str {
        "UiPlugin"
    }

    fn build(&self, app: &mut App) {
        app.insert_resource(Session::default())
            .add_system(Stage::Update, ui_router_system);
    }
}
