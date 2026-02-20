use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

/// Return "now" in RFC3339 format for persisted metadata and API timestamps.
pub(crate) fn now_str() -> String {
    Utc::now().to_rfc3339()
}

/// Convert the public timeframe token into a UTC cutoff timestamp.
pub(crate) fn timeframe_cutoff(timeframe: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    let cutoff = match timeframe {
        "30d" => now - Duration::days(30),
        "90d" => now - Duration::days(90),
        "180d" => now - Duration::days(180),
        "7d" => now - Duration::days(7),
        _ => return Err(anyhow::anyhow!("unsupported timeframe: {}", timeframe)),
    };
    Ok(cutoff)
}
