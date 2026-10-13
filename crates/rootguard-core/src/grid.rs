//! 空间定义（SPEC 2.2 / 2.5 / 7.2 / 9）：网格常量、碰撞半宽与坐标换算。
//!
//! 这是代码中唯一允许出现空间数值的地方（AGENT.md 3.3）；玩法数值全部在 `assets/data/`。

use trellia_core::Vec2;

/// 网格与车道几何（SPEC 5.2 `Grid`）。纯函数集合，同时作为资源插入以便查询。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Grid;

impl Grid {
    /// 行数。
    pub const ROWS: u8 = 5;
    /// 列数。
    pub const COLS: u8 = 9;
    /// 网格原点（左上，SPEC 2.2）。
    pub const ORIGIN: Vec2 = Vec2::new(260.0, 90.0);
    /// 格子尺寸。
    pub const CELL_SIZE: Vec2 = Vec2::new(100.0, 115.0);
    /// 网格右边界 x。
    pub const RIGHT_X: f32 = 1160.0;
    /// 敌人生成基准 x（另加 `GameRng.gen_range(0..SPAWN_JITTER_PX)`）。
    pub const SPAWN_X: f32 = 1240.0;
    /// 敌人生成 x 抖动上限（不含）。
    pub const SPAWN_JITTER_PX: u32 = 40;
    /// 敌人失败线：x 小于此值。
    pub const FAIL_X: f32 = 250.0;
    /// Hose Burst 停放 x（仅呈现）。
    pub const HOSE_X: f32 = 235.0;
    /// 顶部 HUD 区域下边界 y。
    pub const HUD_BOTTOM_Y: f32 = 80.0;
    /// 底部提示区域上边界 y。
    pub const HINT_TOP_Y: f32 = 680.0;
    /// 敌人碰撞区间半宽（SPEC 2.5）。
    pub const ENEMY_HALF_WIDTH: f32 = 20.0;
    /// 弹丸碰撞区间半宽（SPEC 2.5）。
    pub const PROJECTILE_HALF_WIDTH: f32 = 6.0;
    /// Shooter 索敌：敌人 x 须大于 `cell_right - SHOOTER_BACK_TOLERANCE`（SPEC 7.2）。
    pub const SHOOTER_BACK_TOLERANCE: f32 = 10.0;
    /// Shooter 索敌：敌人 x 不大于此值（SPEC 7.2）。
    pub const SHOOTER_MAX_X: f32 = 1240.0;
    /// 弹丸生成位置相对格子中心的偏移（SPEC 7.2）。
    pub const MUZZLE_OFFSET: Vec2 = Vec2::new(30.0, -20.0);
    /// 弹丸越过此 x 即销毁（逻辑宽度 1280 外，覆盖全部生成位置，ADR-002）。
    pub const PROJECTILE_DESPAWN_X: f32 = 1300.0;
    /// Gust 推送后的 x 上限（SPEC 7.2）。
    pub const GUST_CLAMP_X: f32 = 1260.0;
    /// 天空 Dew 起始 y（SPEC 9）。
    pub const SKY_DEW_START_Y: f32 = -40.0;
    /// Dew 点击碰撞圆半径（SPEC 9）。
    pub const DEW_COLLECT_RADIUS: f32 = 36.0;
    /// 防御者占位尺寸（SPEC 12.2）。
    pub const DEFENDER_SIZE: Vec2 = Vec2::new(80.0, 90.0);
    /// 敌人占位尺寸（SPEC 12.2）。
    pub const ENEMY_SIZE: Vec2 = Vec2::new(50.0, 90.0);
    /// Producer 产出的 Dew 相对格子中心的最大偏移（SPEC 7.2，`GameRng` ±30）。
    pub const PRODUCER_SCATTER_PX: i32 = 30;

    /// 格子中心（SPEC 2.2：x = 310 + 100 col，y = 147.5 + 115 row）。
    #[must_use]
    pub fn cell_center(row: u8, col: u8) -> Vec2 {
        Vec2::new(
            Self::ORIGIN.x + Self::CELL_SIZE.x * (f32::from(col) + 0.5),
            Self::ORIGIN.y + Self::CELL_SIZE.y * (f32::from(row) + 0.5),
        )
    }

    /// 第 `col` 列左边界 x。
    #[must_use]
    pub fn cell_left(col: u8) -> f32 {
        Self::ORIGIN.x + Self::CELL_SIZE.x * f32::from(col)
    }

    /// 第 `col` 列右边界 x。
    #[must_use]
    pub fn cell_right(col: u8) -> f32 {
        Self::cell_left(col) + Self::CELL_SIZE.x
    }

    /// 第 `row` 行中心 y。
    #[must_use]
    pub fn row_center_y(row: u8) -> f32 {
        Self::ORIGIN.y + Self::CELL_SIZE.y * (f32::from(row) + 0.5)
    }

    /// 第 `row` 行底部 y（SPEC 2.2 `sort_y`：205 + 115 row）。
    #[must_use]
    pub fn sort_y(row: u8) -> f32 {
        Self::ORIGIN.y + Self::CELL_SIZE.y * f32::from(row + 1)
    }

    /// 逻辑坐标所在格子 `(row, col)`；网格外为 `None`。
    #[must_use]
    pub fn cell_at(pos: Vec2) -> Option<(u8, u8)> {
        let col = Self::col_at_x(pos.x)?;
        let row = Self::lane_at_y(pos.y)?;
        Some((row, col))
    }

    /// 逻辑 y 所在车道。
    #[must_use]
    pub fn lane_at_y(y: f32) -> Option<u8> {
        (0..Self::ROWS).find(|&r| {
            let top = Self::ORIGIN.y + Self::CELL_SIZE.y * f32::from(r);
            y >= top && y < top + Self::CELL_SIZE.y
        })
    }

    /// 逻辑 x 所在列。
    #[must_use]
    pub fn col_at_x(x: f32) -> Option<u8> {
        (0..Self::COLS).find(|&c| x >= Self::cell_left(c) && x < Self::cell_right(c))
    }

    /// 敌人碰撞区间 `[x - 20, x + 20]` 是否与第 `col` 列（向左扩展 `extra_left` 列、向右扩展 `extra_right` 列）相交。
    #[must_use]
    pub fn enemy_overlaps_cols(x: f32, col: u8, extra_left: u8, extra_right: u8) -> bool {
        let left = Self::cell_left(col) - Self::CELL_SIZE.x * f32::from(extra_left);
        let right = Self::cell_right(col) + Self::CELL_SIZE.x * f32::from(extra_right);
        x - Self::ENEMY_HALF_WIDTH <= right && x + Self::ENEMY_HALF_WIDTH >= left
    }

    /// 敌人是否"到达"第 `col` 列的防御者（SPEC 2.5：`x - 20 ≤ cell_right` 且 `x > cell_left`）。
    #[must_use]
    pub fn enemy_reaches_col(x: f32, col: u8) -> bool {
        x - Self::ENEMY_HALF_WIDTH <= Self::cell_right(col) && x > Self::cell_left(col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_2_2_coordinates() {
        assert_eq!(Grid::cell_center(0, 0), Vec2::new(310.0, 147.5));
        assert_eq!(Grid::cell_center(4, 8), Vec2::new(1110.0, 607.5));
        assert_eq!(Grid::sort_y(0), 205.0);
        assert_eq!(Grid::sort_y(4), 665.0);
        assert_eq!(Grid::cell_right(8), Grid::RIGHT_X);
        assert_eq!(Grid::cell_at(Vec2::new(310.0, 147.5)), Some((0, 0)));
        assert_eq!(Grid::cell_at(Vec2::new(1159.0, 664.0)), Some((4, 8)));
        assert_eq!(Grid::cell_at(Vec2::new(1160.0, 300.0)), None);
        assert_eq!(Grid::cell_at(Vec2::new(300.0, 80.0)), None);
    }

    #[test]
    fn reach_interval() {
        // 第 0 列 [260, 360]：x - 20 ≤ 360 且 x > 260
        assert!(Grid::enemy_reaches_col(380.0, 0));
        assert!(!Grid::enemy_reaches_col(380.5, 0));
        assert!(!Grid::enemy_reaches_col(260.0, 0));
    }
}
