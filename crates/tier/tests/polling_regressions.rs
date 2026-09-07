#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tempfile::{NamedTempFile, tempdir};
use tier::{ConfigLoader, PollingWatcher, ReloadEvent, ReloadHandle};

#[derive(Clone, Deserialize, Serialize)]
struct Config {
    content: Option<String>,
}

fn write_config(path: &Path, content: &str, modified_seconds: u64) {
    let mut file = NamedTempFile::new_in(path.parent().expect("config parent directory"))
        .expect("temporary config file");
    file.write_all(content.as_bytes()).expect("write config");
    file.as_file()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(modified_seconds))
        .expect("set deterministic modification time");
    file.persist(path).expect("replace config atomically");
}

fn watch(
    config_path: &Path,
    watched_path: &Path,
) -> (ReloadHandle<Config>, PollingWatcher, Receiver<ReloadEvent>) {
    let config_path = config_path.to_path_buf();
    let handle = ReloadHandle::new(move || {
        ConfigLoader::new(Config {
            content: fs::read_to_string(&config_path).ok(),
        })
        .load()
    })
    .expect("initial load");
    let events = handle.subscribe();
    let watcher = handle.start_polling([watched_path], Duration::from_millis(10));
    (handle, watcher, events)
}

fn assert_reloaded(
    handle: &ReloadHandle<Config>,
    events: &Receiver<ReloadEvent>,
    expected: Option<&str>,
) {
    let event = events
        .recv_timeout(Duration::from_secs(2))
        .expect("polling reload event");
    assert!(matches!(event, ReloadEvent::Applied(_)), "{event:?}");
    assert_eq!(handle.config().content.as_deref(), expected);
}

#[test]
fn polling_observes_symlink_target_modifications() {
    let dir = tempdir().expect("temporary directory");
    let target = dir.path().join("target");
    let link = dir.path().join("config");
    write_config(&target, "first", 1);
    symlink("target", &link).expect("relative file symlink");
    let (handle, watcher, events) = watch(&link, &link);

    write_config(&target, "second", 2);

    assert_reloaded(&handle, &events, Some("second"));
    watcher.stop();
}

#[test]
fn polling_observes_replaced_links_with_identical_target_mtimes() {
    let dir = tempdir().expect("temporary directory");
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    let link = dir.path().join("config");
    let replacement = dir.path().join("replacement");
    write_config(&first, "first", 1);
    write_config(&second, "second", 1);
    symlink(&first, &link).expect("initial symlink");
    symlink(&second, &replacement).expect("replacement symlink");
    let (handle, watcher, events) = watch(&link, &link);

    fs::rename(&replacement, &link).expect("replace symlink atomically");

    assert_reloaded(&handle, &events, Some("second"));
    watcher.stop();
}

#[test]
fn polling_observes_dangling_target_creation_removal_and_recreation() {
    let dir = tempdir().expect("temporary directory");
    let target = dir.path().join("target");
    let link = dir.path().join("config");
    symlink(&target, &link).expect("dangling symlink");
    let (handle, watcher, events) = watch(&link, &link);

    write_config(&target, "first", 1);
    assert_reloaded(&handle, &events, Some("first"));

    fs::remove_file(&target).expect("remove target");
    assert_reloaded(&handle, &events, None);

    write_config(&target, "second", 1);
    assert_reloaded(&handle, &events, Some("second"));
    watcher.stop();
}

#[test]
fn polling_follows_directory_links_and_stops_at_recursive_links() {
    let dir = tempdir().expect("temporary directory");
    let target_dir = dir.path().join("target");
    fs::create_dir_all(target_dir.join("nested")).expect("target directory");
    let target = target_dir.join("nested/config");
    let link = dir.path().join("linked-directory");
    write_config(&target, "first", 1);
    symlink(&target_dir, &link).expect("directory symlink");
    symlink(&target_dir, target_dir.join("loop")).expect("recursive symlink");
    let (handle, watcher, events) = watch(&link.join("nested/config"), &link);

    write_config(&target, "second", 2);

    assert_reloaded(&handle, &events, Some("second"));
    watcher.stop();
}

#[test]
fn polling_follows_file_links_inside_watched_directories() {
    let dir = tempdir().expect("temporary directory");
    let watched_dir = dir.path().join("watched");
    fs::create_dir(&watched_dir).expect("watched directory");
    let target = dir.path().join("external-config");
    let link = watched_dir.join("config");
    write_config(&target, "first", 1);
    symlink(&target, &link).expect("file symlink");
    let (handle, watcher, events) = watch(&link, &watched_dir);

    write_config(&target, "second", 2);

    assert_reloaded(&handle, &events, Some("second"));
    watcher.stop();
}
