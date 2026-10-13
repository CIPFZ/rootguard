//! 冒烟测试：App 能在 headless 下组装并停留在标题场景。

use rootguard_app::save::SaveLocation;
use rootguard_app::{TitleScene, build_app_full};
use trellia::prelude::*;

#[test]
fn app_boots_to_title_headless() {
    let dir = std::env::temp_dir().join(format!("rootguard-smoke-{}", std::process::id()));
    let mut runner = HeadlessRunner::new(build_app_full(TitleScene, SaveLocation::Dir(dir)));
    runner.frames(3);
    let world = runner.app().world();
    assert_eq!(
        world.resources.expect::<SceneStack>().top_name(),
        Some("Title")
    );
    assert!(!runner.last_draw_list().is_empty());
}
