//! 选卡场景（SPEC 3.4 / 11.3）：左侧已解锁卡牌，右侧卡槽；“开始”至少 1 张。`Forced` 关跳过本场景。

use rootguard_core::events::UiAction;
use rootguard_core::registry::{DefenderRegistry, LevelRegistry};
use rootguard_core::resources::Progress;
use rootguard_core::{DefenderId, LevelId};
use trellia::egui;
use trellia::prelude::*;

use crate::i18n::{t, tf};
use crate::scenes::{button_at, draw_menu, send};
use crate::sfx::{self, Track};

/// 卡牌网格每行数量。
const GRID_COLS: usize = 4;

/// 选卡场景。
pub struct SeedSelectScene {
    level: LevelId,
    chosen: Vec<DefenderId>,
}

impl SeedSelectScene {
    /// 为关卡创建。
    #[must_use]
    pub fn new(level: LevelId) -> Self {
        Self {
            level,
            chosen: Vec::new(),
        }
    }

    fn slots(world: &World, level: LevelId) -> usize {
        world
            .resources
            .get::<LevelRegistry>()
            .and_then(|l| l.get(level))
            .map_or(0, |l| usize::from(l.card_slots))
    }
}

fn card_label(world: &World, id: DefenderId) -> String {
    let cost = world
        .resources
        .get::<DefenderRegistry>()
        .and_then(|d| d.get(id))
        .map_or(0, |d| d.cost);
    format!("{}\n{cost}", t(world, &format!("defender.{id}.name")))
}

impl Scene for SeedSelectScene {
    fn name(&self) -> &'static str {
        "SeedSelect"
    }

    fn on_enter(&mut self, world: &mut World) {
        sfx::play_music(world, Track::Title);
    }

    fn update(&mut self, world: &mut World) {
        if world
            .resources
            .expect::<Input>()
            .key_just_pressed(Key::Escape)
        {
            send(world, UiAction::BackToLevelSelect);
        }
    }

    fn draw(&mut self, world: &mut World) {
        let title = format!("{} · {}", t(world, "seed_select.title"), self.level);
        let hint = t(world, "seed_select.hint");
        draw_menu(world, &title, Some(&hint));
        let max = Self::slots(world, self.level);
        let header = tf(
            world,
            "seed_select.slots",
            &[
                ("n", &self.chosen.len().to_string()),
                ("max", &max.to_string()),
            ],
        );
        world.resources.expect_mut::<DrawList>().text(
            Layer::OVERLAY,
            Vec2::new(760.0, 190.0),
            header,
            22.0,
            Color::WHITE,
        );
    }

    fn ui(&mut self, world: &mut World, ctx: &egui::Context) {
        let max = Self::slots(world, self.level);
        let unlocked: Vec<DefenderId> = world
            .resources
            .get::<Progress>()
            .map(|p| p.unlocked.iter().copied().collect())
            .unwrap_or_default();
        // 左侧：已解锁卡牌
        for (i, id) in unlocked.iter().copied().enumerate() {
            let rect = Rect::from_min_size(
                Vec2::new(
                    60.0 + 150.0 * (i % GRID_COLS) as f32,
                    210.0 + 90.0 * (i / GRID_COLS) as f32,
                ),
                Vec2::new(140.0, 80.0),
            );
            let picked = self.chosen.contains(&id);
            let enabled = !picked && self.chosen.len() < max;
            if button_at(
                world,
                ctx,
                &format!("seed_{id}"),
                rect,
                &card_label(world, id),
                enabled,
            ) {
                self.chosen.push(id);
            }
        }
        // 右侧：卡槽
        let mut remove = None;
        for (i, id) in self.chosen.iter().copied().enumerate() {
            let rect = Rect::from_min_size(
                Vec2::new(
                    760.0 + 230.0 * (i % 2) as f32,
                    225.0 + 70.0 * (i / 2) as f32,
                ),
                Vec2::new(210.0, 60.0),
            );
            if button_at(
                world,
                ctx,
                &format!("slot_{i}"),
                rect,
                &card_label(world, id),
                true,
            ) {
                remove = Some(i);
            }
        }
        if let Some(i) = remove {
            self.chosen.remove(i);
        }
        let start = Rect::from_min_size(Vec2::new(1040.0, 630.0), Vec2::new(200.0, 54.0));
        if button_at(
            world,
            ctx,
            "seed_start",
            start,
            &t(world, "seed_select.start"),
            !self.chosen.is_empty(),
        ) {
            send(world, UiAction::ConfirmSeeds(self.chosen.clone()));
        }
        let back = Rect::from_min_size(Vec2::new(40.0, 630.0), Vec2::new(180.0, 54.0));
        if button_at(
            world,
            ctx,
            "seed_back",
            back,
            &t(world, "seed_select.back"),
            true,
        ) {
            send(world, UiAction::BackToLevelSelect);
        }
    }
}
