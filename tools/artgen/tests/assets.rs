//! ART-SPEC 9.2 / 11 A0：提交的 `assets/skins/garden/` 必须与 `art/` 重新生成的结果一致（确定性输出）。

use std::path::Path;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn art_assets_up_to_date() {
    let built = artgen::build(root()).expect("artgen 构建（含 ART-SPEC 9.2 全部校验）");
    let problems = artgen::diff(root(), &built).expect("比较");
    assert!(
        problems.is_empty(),
        "assets/skins/garden 与 art/ 不一致，请运行 `cargo run --release -p artgen -- build`：\n{}",
        problems.join("\n")
    );
    // 同一输入两次构建逐字节相同
    let again = artgen::build(root()).expect("再次构建");
    assert!(again.files == built.files, "两次构建结果不同（非确定性）");
}
