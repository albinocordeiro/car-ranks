use serde::Serialize;

/// Response for sampling configuration bootstrap endpoint.
#[derive(Debug, Serialize)]
pub(crate) struct SamplingConfigResponse {
    pub(crate) generated_at: String,
    pub(crate) platform: String,
    pub(crate) source: String,
    pub(crate) read_only: bool,
    pub(crate) batch_upload: BatchUploadConfig,
    pub(crate) sampling_profiles: Vec<SamplingProfile>,
    pub(crate) kpi_refresh: KpiRefreshConfig,
    pub(crate) feature_flags: FeatureFlags,
}

#[derive(Debug, Serialize)]
pub(crate) struct BatchUploadConfig {
    pub(crate) default_interval_seconds: i64,
    pub(crate) min_interval_seconds: i64,
    pub(crate) max_interval_seconds: i64,
    pub(crate) next_upload_after_seconds: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct SamplingProfile {
    pub(crate) mode: String,
    pub(crate) sample_interval_seconds: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct KpiRefreshConfig {
    pub(crate) active_vehicle_interval_seconds: i64,
    pub(crate) daily_rebuild_interval_seconds: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct FeatureFlags {
    pub(crate) smartcar_enabled: bool,
    pub(crate) remote_commands_enabled: bool,
}
