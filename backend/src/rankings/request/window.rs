use crate::RankingsQuery;

/// Normalized pagination and scope controls for rankings reads.
pub(in crate::rankings) struct RankingsWindow {
    pub(in crate::rankings) timeframe: String,
    pub(in crate::rankings) temperature_bin: String,
    pub(in crate::rankings) limit: i64,
    pub(in crate::rankings) offset: i64,
}

/// Resolves default timeframe/bin values and pagination bounds.
pub(in crate::rankings) fn normalize_rankings_window(params: &RankingsQuery) -> RankingsWindow {
    RankingsWindow {
        timeframe: params
            .timeframe
            .clone()
            .unwrap_or_else(|| "30d".to_string()),
        temperature_bin: params
            .temperature_bin
            .clone()
            .unwrap_or_else(|| "all".to_string()),
        limit: params.limit.unwrap_or(25).clamp(1, 100),
        offset: params.offset.unwrap_or(0).max(0),
    }
}
