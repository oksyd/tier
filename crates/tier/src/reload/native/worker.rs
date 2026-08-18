use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::event::{WatchMessage, reload_deadline_for_event};
use super::target::WatchTarget;
use crate::reload::{ReloadFailurePolicy, ReloadHandle, ReloadOptions};

pub(super) fn run_native_watch_loop<T>(
    handle: ReloadHandle<T>,
    targets: Vec<WatchTarget>,
    rx: Receiver<WatchMessage>,
    debounce: Duration,
    options: ReloadOptions,
) where
    T: Send + Sync + 'static,
{
    loop {
        let Some(deadline) = wait_for_first_reload_deadline(&handle, &targets, &rx, debounce)
        else {
            return;
        };
        let Some(()) = collect_debounced_events(&handle, &targets, &rx, debounce, deadline) else {
            return;
        };

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
) -> Option<Instant> {
    loop {
        match rx.recv() {
            Ok(WatchMessage::Stop) | Err(_) => return None,
            Ok(WatchMessage::Event(event)) => {
                if let Some(deadline) = reload_deadline_for_event(handle, targets, event, debounce)
                {
                    return Some(deadline);
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
) -> Option<()> {
    loop {
        let timeout = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(timeout) {
            Ok(WatchMessage::Stop) | Err(RecvTimeoutError::Disconnected) => return None,
            Err(RecvTimeoutError::Timeout) => return Some(()),
            Ok(WatchMessage::Event(event)) => {
                if let Some(next_deadline) =
                    reload_deadline_for_event(handle, targets, event, debounce)
                {
                    deadline = next_deadline;
                }
            }
        }
    }
}
