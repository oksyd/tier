use std::time::{Duration, Instant};

use notify::{Event, EventKind};

use crate::reload::ReloadHandle;

use super::target::{WatchTarget, absolutize_event_path};

pub(super) enum WatchMessage {
    Event(notify::Result<Event>),
    Stop,
}

pub(super) fn reload_deadline_for_event<T>(
    handle: &ReloadHandle<T>,
    targets: &[WatchTarget],
    event: notify::Result<Event>,
    debounce: Duration,
) -> Option<Instant> {
    match event {
        Ok(event) if event_requires_reload(&event, targets) => Some(Instant::now() + debounce),
        Ok(_) => None,
        Err(error) => {
            handle.record_error_message(format!("watch error: {error}"));
            None
        }
    }
}

fn event_requires_reload(event: &Event, targets: &[WatchTarget]) -> bool {
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }

    if event.paths.is_empty() {
        return true;
    }

    event
        .paths
        .iter()
        .filter_map(|path| absolutize_event_path(path))
        .any(|path| {
            targets
                .iter()
                .any(|target| target.matches_event_path(&path))
        })
}
