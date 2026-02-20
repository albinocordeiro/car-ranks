use crate::TelemetryBatchRequest;

/// Source metadata carried through ingest persistence and idempotency paths.
pub(super) struct SourceContext {
    pub(super) source_upper: String,
    pub(super) source_account_id: String,
}

/// Derives source fields from the validated envelope and optional client block.
pub(super) fn build_source_context(
    payload: &TelemetryBatchRequest,
    source_upper: String,
) -> SourceContext {
    let source_account_id = payload
        .client
        .as_ref()
        .and_then(|client| client.adapter_fingerprint.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Kept for future ingestion provenance expansion; this pass only materializes
    // adapter fingerprint into persistent source-account metadata.
    let _client_app_version = payload
        .client
        .as_ref()
        .and_then(|client| client.app_version.clone());

    SourceContext {
        source_upper,
        source_account_id,
    }
}
