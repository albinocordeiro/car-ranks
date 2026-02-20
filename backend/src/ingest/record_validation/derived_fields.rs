use crate::{TelemetryRecord, derive_temperature_bin};

/// Derives the temperature bin used for downstream KPI materialization.
pub(super) fn derive_temperature_bin_for_record(record: &TelemetryRecord) -> Option<String> {
    record.temperature_bin.clone().or_else(|| {
        match (record.signal_key.as_str(), record.value_number) {
            ("environment.ambient_temp_c", Some(temperature)) => {
                Some(derive_temperature_bin(temperature).to_string())
            }
            _ => None,
        }
    })
}
