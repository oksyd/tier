use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::event::{EventReaction, WatchMessage, reject_watch_error, reload_deadline_for_event};
use super::target::{WatchTarget, collect_watch_registrations, prepare_watch_targets};
use crate::reload::{ReloadFailurePolicy, ReloadHandle, ReloadOptions};
use notify::{RecommendedWatcher, Watcher};
use std::path::PathBuf;

pub(super) fn run_native_watch_loop<T>(
    handle: ReloadHandle<T>,
    paths: Vec<PathBuf>,
    mut targets: Vec<WatchTarget>,
    mut watcher: RecommendedWatcher,
    rx: Receiver<WatchMessage>,
    debounce: Duration,
    options: ReloadOptions,
) where
    T: Send + Sync + 'static,
{
    loop {
        let Some(deadline) =
            wait_for_first_reload_deadline(&handle, &targets, &rx, debounce, &options)
        else {
            return;
        };
        let Some(()) =
            collect_debounced_events(&handle, &targets, &rx, debounce, deadline, &options)
        else {
            return;
        };

        // Link replacements can point outside the original watch roots. Rebuild
        // subscriptions before loading the new target and observing future edits.
        let mut watch_error = None;
        match prepare_watch_targets(paths.clone()) {
            Ok(next) => {
                let old = collect_watch_registrations(&targets);
                let new = collect_watch_registrations(&next);
                // An unchanged path may now refer to a new directory inode.
                // Reset registrations before loading so the next snapshot and
                // subsequent events both refer to the current filesystem tree.
                for registration in &old {
                    let _ = watcher.unwatch(&registration.root);
                }
                for registration in &new {
                    if let Err(error) = registration.register(&mut watcher) {
                        watch_error = Some(format!("watch error: {error}"));
                    }
                }
                targets = next;
            }
            Err(error) => watch_error = Some(error.to_string()),
        }
        if let Some(error) = watch_error {
            if reject_watch_error(&handle, error, &options) {
                return;
            }
            continue;
        }

        if handle.reload_with_options(&options).is_err()
            && matches!(options.on_error, ReloadFailurePolicy::StopWatcher)
        {
            return;
        }
    }
}

fn wait_for_first_reload_deadline<T>(
    handle: &ReloadHandle<T>,
    targets: &[WatchTarget],
    rx: &Receiver<WatchMessage>,
    debounce: Duration,
    options: &ReloadOptions,
) -> Option<Instant> {
    loop {
        match rx.recv() {
            Ok(WatchMessage::Stop) | Err(_) => return None,
            Ok(WatchMessage::Event(event)) => {
                match reload_deadline_for_event(handle, targets, event, debounce, options) {
                    EventReaction::Reload(deadline) => return Some(deadline),
                    EventReaction::Stop => return None,
                    EventReaction::Ignore => {}
                }
            }
        }
    }
}

fn collect_debounced_events<T>(
    handle: &ReloadHandle<T>,
    targets: &[WatchTarget],
    rx: &Receiver<WatchMessage>,
    debounce: Duration,
    mut deadline: Instant,
    options: &ReloadOptions,
) -> Option<()> {
    loop {
        let timeout = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(timeout) {
            Ok(WatchMessage::Stop) | Err(RecvTimeoutError::Disconnected) => return None,
            Err(RecvTimeoutError::Timeout) => return Some(()),
            Ok(WatchMessage::Event(event)) => {
                match reload_deadline_for_event(handle, targets, event, debounce, options) {
                    EventReaction::Reload(next_deadline) => deadline = next_deadline,
                    EventReaction::Stop => return None,
                    EventReaction::Ignore => {}
                }
            }
        }
    }
}
