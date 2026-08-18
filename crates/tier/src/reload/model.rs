use super::ReloadSummary;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Policy applied when a background watcher encounters a reload failure.
pub enum ReloadFailurePolicy {
    /// Keep the last good configuration and continue watching for future changes.
    #[default]
    KeepLastGood,
    /// Keep the last good configuration and stop the background watcher.
    StopWatcher,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Options controlling watcher-side reload behavior.
pub struct ReloadOptions {
    /// Behavior applied after a failed reload.
    pub on_error: ReloadFailurePolicy,
    /// Whether to emit success events even when the effective configuration did not change.
    pub emit_unchanged: bool,
}

impl Default for ReloadOptions {
    fn default() -> Self {
        Self {
            on_error: ReloadFailurePolicy::KeepLastGood,
            emit_unchanged: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Structured details about a rejected reload attempt.
pub struct ReloadFailure {
    /// Human-readable error message.
    pub error: String,
    /// Whether the previous configuration snapshot was preserved.
    pub last_good_retained: bool,
    /// Whether the watcher that observed the error stopped after the failure.
    pub watcher_stopped: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// Structured event emitted after each successful or rejected reload attempt.
pub enum ReloadEvent {
    /// A reload was applied successfully.
    Applied(ReloadSummary),
    /// A reload failed and the previous configuration was kept.
    Rejected(ReloadFailure),
}
