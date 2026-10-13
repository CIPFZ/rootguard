//! G0 冒烟测试：App 能在 headless 下组装并停留在标题场景。

use trellia::prelude::*;

#[test]
fn app_boots_to_title_headless() {
    let mut runner = trellia::run_headless(rootguard_app::build_app());
    runner.frames(3);
    let world = runner.app().world();
    assert_eq!(
        world.resources.expect::<SceneStack>().top_name(),
        Some("Title")
    );
    assert!(!runner.last_draw_list().is_empty());
}
