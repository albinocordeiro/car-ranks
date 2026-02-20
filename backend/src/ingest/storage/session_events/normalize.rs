use anyhow::Context;

use crate::map_session_event;

/// Normalizes raw session event labels into persisted type enums.
pub(super) fn normalize_session_event(
    event_type: &str,
) -> anyhow::Result<(&'static str, &'static str)> {
    map_session_event(event_type)
        .ok_or_else(|| anyhow::anyhow!("unsupported session event type {}", event_type))
        .context("failed to map session event")
}
