#![cfg(all(unix, feature = "watch"))]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use serde_json::{Value, json};
use std::{fs, os::unix::fs::symlink, path::Path, sync::mpsc::Receiver, time::Duration};
use tier::{ConfigLoader, NativeWatcher, ReloadEvent, ReloadHandle};

fn watch(
    read: &Path,
    watched: &Path,
) -> (ReloadHandle<Value>, NativeWatcher, Receiver<ReloadEvent>) {
    let read = read.to_path_buf();
    let handle = ReloadHandle::new(move || {
        ConfigLoader::new(json!({"content":fs::read_to_string(&read).ok()})).load()
    })
    .unwrap();
    let events = handle.subscribe();
    let watcher = handle
        .start_native([watched], Duration::from_millis(20))
        .unwrap();
    (handle, watcher, events)
}
fn applied(handle: &ReloadHandle<Value>, events: &Receiver<ReloadEvent>, expected: &str) {
    let event = events
        .recv_timeout(Duration::from_secs(3))
        .expect("native reload");
    assert!(matches!(event, ReloadEvent::Applied(_)), "{event:?}");
    assert_eq!(handle.config()["content"], expected);
}
#[test]
fn native_watches_target_edits_and_atomic_replacements() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target");
    let link = dir.path().join("config");
    fs::write(&target, "first").unwrap();
    symlink("target", &link).unwrap();
    let (handle, watcher, events) = watch(&link, &link);
    fs::write(&target, "second").unwrap();
    applied(&handle, &events, "second");
    let replacement = dir.path().join("replacement");
    fs::write(&replacement, "third").unwrap();
    fs::rename(replacement, target).unwrap();
    applied(&handle, &events, "third");
    watcher.stop();
}
#[test]
fn native_updates_subscriptions_after_link_retargets_to_another_directory() {
    let dir = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let first = dir.path().join("first");
    let second = other.path().join("second");
    let link = dir.path().join("config");
    fs::write(&first, "first").unwrap();
    fs::write(&second, "second").unwrap();
    symlink(&first, &link).unwrap();
    let (handle, watcher, events) = watch(&link, &link);
    let replacement = dir.path().join("replacement");
    symlink(&second, &replacement).unwrap();
    fs::rename(replacement, &link).unwrap();
    applied(&handle, &events, "second");
    fs::write(&second, "third").unwrap();
    applied(&handle, &events, "third");
    watcher.stop();
}
#[test]
fn native_follows_symlinks_in_parent_directories() {
    let dir = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let link = dir.path().join("directory");
    let file = target.path().join("config");
    fs::write(&file, "first").unwrap();
    symlink(target.path(), &link).unwrap();
    let (handle, watcher, events) = watch(&link.join("config"), &link.join("config"));
    fs::write(&file, "second").unwrap();
    applied(&handle, &events, "second");
    watcher.stop();
}
#[test]
fn native_watches_dangling_link_target_creation() {
    let dir = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let target = other.path().join("target");
    let link = dir.path().join("config");
    symlink(&target, &link).unwrap();
    let (handle, watcher, events) = watch(&link, &link);
    fs::write(&target, "created").unwrap();
    applied(&handle, &events, "created");
    watcher.stop();
}
