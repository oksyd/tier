use std::time::{Duration, Instant};

use notify::{Event, EventKind};

use crate::reload::{ReloadEvent, ReloadFailure, ReloadFailurePolicy, ReloadHandle, ReloadOptions};

use super::target::{WatchTarget, absolutize_event_path};

pub(super) enum WatchMessage {
    Event(notify::Result<Event>),
    Stop,
}

pub(super) enum EventReaction {
    Ignore,
    Reload(Instant),
    Stop,
}

pub(super) fn reload_deadline_for_event<T>(
    handle: &ReloadHandle<T>,
    targets: &[WatchTarget],
    event: notify::Result<Event>,
    debounce: Duration,
    options: &ReloadOptions,
) -> EventReaction {
    match event {
        Ok(event) if event_requires_reload(&event, targets) => {
            EventReaction::Reload(Instant::now() + debounce)
        }
        Ok(_) => EventReaction::Ignore,
        Err(error) => {
            if reject_watch_error(handle, format!("watch error: {error}"), options) {
                EventReaction::Stop
            } else {
                EventReaction::Ignore
            }
        }
    }
}

pub(super) fn reject_watch_error<T>(
    handle: &ReloadHandle<T>,
    message: String,
    options: &ReloadOptions,
) -> bool {
    let watcher_stopped = matches!(options.on_error, ReloadFailurePolicy::StopWatcher);
    handle.record_error_message(message.clone());
    handle.emit_event(ReloadEvent::Rejected(ReloadFailure {
        error: message,
        last_good_retained: true,
        watcher_stopped,
    }));
    watcher_stopped
}

fn event_requires_reload(event: &Event, targets: &[WatchTarget]) -> bool {
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }

    if event.need_rescan() || event.paths.is_empty() {
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

#[cfg(test)]
mod tests {
    use super::{EventReaction, reload_deadline_for_event};
    use crate::{ConfigLoader, ReloadEvent, ReloadFailurePolicy, ReloadHandle, ReloadOptions};
    use std::time::Duration;

    #[test]
    fn backend_errors_emit_rejections_and_honor_failure_policy() -> Result<(), crate::ConfigError> {
        for policy in [
            ReloadFailurePolicy::KeepLastGood,
            ReloadFailurePolicy::StopWatcher,
        ] {
            let handle =
                ReloadHandle::new(|| ConfigLoader::new(serde_json::json!({"port": 3000})).load())?;
            let events = handle.subscribe();
            let reaction = reload_deadline_for_event(
                &handle,
                &[],
                Err(notify::Error::generic("watch backend failed")),
                Duration::ZERO,
                &ReloadOptions {
                    on_error: policy,
                    ..ReloadOptions::default()
                },
            );
            let stopped = matches!(policy, ReloadFailurePolicy::StopWatcher);
            assert_eq!(matches!(reaction, EventReaction::Stop), stopped);
            assert!(handle.last_error().is_some());
            assert!(
                matches!(events.try_recv(), Ok(ReloadEvent::Rejected(failure))
                if failure.last_good_retained && failure.watcher_stopped == stopped
                && failure.error.contains("watch backend failed"))
            );
            assert_eq!(handle.config()["port"], 3000);
        }
        Ok(())
    }
}
