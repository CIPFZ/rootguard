//! `rootguard-content` 集成测试：真实数据全部通过校验（SPEC 16.2 `every_level_loads_and_validates`），
//! 以及校验失败时信息包含文件路径与字段名（SPEC 4.5）。

use std::path::{Path, PathBuf};

use rootguard_content::{ContentError, load_from};

fn assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

/// 把真实数据复制到临时目录，供破坏性测试修改。
fn copy_assets(tag: &str) -> PathBuf {
    let dst = std::env::temp_dir().join(format!("rootguard-content-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dst);
    copy_dir(&assets_root().join("data"), &dst.join("data"));
    dst
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("创建临时目录");
    for entry in std::fs::read_dir(src).expect("读取数据目录") {
        let entry = entry.expect("目录项");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("复制文件");
        }
    }
}

fn invalid_messages(root: &Path) -> String {
    match load_from(root) {
        Err(ContentError::Invalid(errors)) => errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
        Err(other) => panic!("期望校验错误，得到 {other}"),
        Ok(_) => panic!("期望校验失败"),
    }
}

#[test]
fn every_level_loads_and_validates() {
    let content = load_from(assets_root()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(content.defenders.len(), 12, "SPEC 7.1：12 种防御者");
    assert_eq!(content.enemies.len(), 9, "SPEC 8.1：8 种敌人 + 1 衍生");
    assert!(!content.levels.is_empty());
}

#[test]
fn invalid_value_reports_path_and_field() {
    let root = copy_assets("value");
    let file = root.join("data/defenders/barkwall.defender.ron");
    let text = std::fs::read_to_string(&file).expect("读取");
    std::fs::write(
        &file,
        text.replace("cooldown_ticks: 1800", "cooldown_ticks: 0"),
    )
    .expect("写入");
    let msg = invalid_messages(&root);
    assert!(
        msg.contains("data/defenders/barkwall.defender.ron"),
        "{msg}"
    );
    assert!(msg.contains("cooldown_ticks"), "{msg}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn level_rules_are_checked() {
    let root = copy_assets("level");
    let file = root.join("data/levels/1-1.level.ron");
    let text = std::fs::read_to_string(&file)
        .expect("读取")
        .replace("card_slots: 6", "card_slots: 9")
        .replace("flag: true", "flag: false")
        .replace("enemy: \"husk\", count: 1", "enemy: \"nobody\", count: 1");
    std::fs::write(&file, text).expect("写入");
    let msg = invalid_messages(&root);
    assert!(msg.contains("card_slots"), "{msg}");
    assert!(msg.contains("最后一波必须为 flag 波"), "{msg}");
    assert!(msg.contains("waves[0].spawns[0].enemy"), "{msg}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn duplicate_ids_are_rejected() {
    let root = copy_assets("dup");
    std::fs::copy(
        root.join("data/enemies/husk.enemy.ron"),
        root.join("data/enemies/husk_copy.enemy.ron"),
    )
    .expect("复制");
    let msg = invalid_messages(&root);
    assert!(msg.contains("husk_copy.enemy.ron"), "{msg}");
    assert!(msg.contains("重复"), "{msg}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn parse_error_contains_path() {
    let root = copy_assets("parse");
    std::fs::write(root.join("data/enemies/husk.enemy.ron"), "Enemy(id: ").expect("写入");
    let err = load_from(&root).expect_err("应解析失败").to_string();
    assert!(err.contains("husk.enemy.ron"), "{err}");
    let _ = std::fs::remove_dir_all(root);
}
