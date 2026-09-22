#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};
use tier::{ConfigLoader, ReloadEvent, ReloadHandle};

fn handle(path: &Path) -> (ReloadHandle<Value>, Receiver<ReloadEvent>) {
    let path = path.to_path_buf();
    let handle = ReloadHandle::new(move || {
        ConfigLoader::new(json!({"content": fs::read_to_string(&path).ok()})).load()
    })
    .unwrap();
    let events = handle.subscribe();
    (handle, events)
}

fn applied(handle: &ReloadHandle<Value>, events: &Receiver<ReloadEvent>, expected: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while handle.config()["content"] != expected {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .expect("configuration should reload");
        assert!(matches!(event, ReloadEvent::Applied(_)), "{event:?}");
    }
}

#[test]
fn polling_observes_replacements_with_preserved_modification_times() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config");
    fs::write(&path, "first").unwrap();
    let modified = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    let (handle, events) = handle(&path);
    let watcher = handle.start_polling([&path], Duration::from_millis(10));
    let replacement = dir.path().join("replacement");
    fs::write(&replacement, "second").unwrap();
    fs::File::options()
        .write(true)
        .open(&replacement)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    fs::rename(replacement, path).unwrap();
    applied(&handle, &events, "second");
    watcher.stop();
}

#[cfg(unix)]
#[test]
fn polling_observes_same_size_replacements_with_preserved_modification_times() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config");
    fs::write(&path, "first").unwrap();
    let modified = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    let (handle, events) = handle(&path);
    let watcher = handle.start_polling([&path], Duration::from_millis(10));
    let replacement = dir.path().join("replacement");
    fs::write(&replacement, "other").unwrap();
    fs::File::options()
        .write(true)
        .open(&replacement)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    fs::rename(replacement, path).unwrap();
    applied(&handle, &events, "other");
    watcher.stop();
}

#[cfg(feature = "watch")]
#[test]
fn native_directory_watch_survives_directory_recreation() {
    let dir = tempfile::tempdir().unwrap();
    let watched = dir.path().join("watched");
    fs::create_dir(&watched).unwrap();
    let path = watched.join("config");
    fs::write(&path, "first").unwrap();
    let (handle, events) = handle(&path);
    let watcher = handle
        .start_native([&watched], Duration::from_millis(10))
        .unwrap();
    fs::remove_dir_all(&watched).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    fs::create_dir(&watched).unwrap();
    fs::write(&path, "second").unwrap();
    applied(&handle, &events, "second");
    fs::write(&path, "third").unwrap();
    applied(&handle, &events, "third");
    watcher.stop();
}

#[cfg(feature = "watch")]
#[test]
fn native_file_watch_survives_parent_directory_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let watched = dir.path().join("watched");
    fs::create_dir(&watched).unwrap();
    let path = watched.join("config");
    fs::write(&path, "first").unwrap();
    let (handle, events) = handle(&path);
    let watcher = handle
        .start_native([&path], Duration::from_millis(10))
        .unwrap();
    fs::rename(&watched, dir.path().join("old")).unwrap();
    fs::create_dir(&watched).unwrap();
    fs::write(&path, "second").unwrap();
    applied(&handle, &events, "second");
    fs::write(&path, "third").unwrap();
    applied(&handle, &events, "third");
    watcher.stop();
}

#[cfg(feature = "watch")]
#[test]
fn native_directory_watch_survives_atomic_directory_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let watched = dir.path().join("watched");
    fs::create_dir(&watched).unwrap();
    let path = watched.join("config");
    fs::write(&path, "first").unwrap();
    let (handle, events) = handle(&path);
    let watcher = handle
        .start_native([&watched], Duration::from_millis(10))
        .unwrap();
    fs::rename(&watched, dir.path().join("old")).unwrap();
    fs::create_dir(&watched).unwrap();
    fs::write(&path, "second").unwrap();
    applied(&handle, &events, "second");
    fs::write(&path, "third").unwrap();
    applied(&handle, &events, "third");
    watcher.stop();
}

#[cfg(all(unix, feature = "watch"))]
#[test]
fn native_resolves_parent_components_in_symlink_destinations() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("nested")).unwrap();
    let path = dir.path().join("config");
    let link = dir.path().join("link");
    fs::write(&path, "first").unwrap();
    std::os::unix::fs::symlink("nested/../config", &link).unwrap();
    let (handle, events) = handle(&link);
    let watcher = handle
        .start_native([&link], Duration::from_millis(10))
        .unwrap();
    fs::write(&path, "second").unwrap();
    applied(&handle, &events, "second");
    watcher.stop();
}

#[cfg(all(unix, feature = "watch"))]
#[test]
fn native_follows_external_file_links_inside_watched_directories() {
    let dir = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let path = external.path().join("config");
    let link = dir.path().join("link");
    fs::write(&path, "first").unwrap();
    std::os::unix::fs::symlink(&path, &link).unwrap();
    let (handle, events) = handle(&link);
    let watcher = handle
        .start_native([dir.path()], Duration::from_millis(10))
        .unwrap();
    fs::write(&path, "second").unwrap();
    applied(&handle, &events, "second");
    watcher.stop();
}
