use super::{TemperatureImpactAccumulator, TemperatureTimestampSnapshot};

impl TemperatureImpactAccumulator {
    /// Applies sparse snapshot fields onto the rolling temperature-impact state.
    ///
    /// Observations can omit unchanged values, so we carry forward the latest
    /// odometer/SOC/temperature until a new reading is present.
    pub(super) fn refresh_latest_snapshot_state(
        &mut self,
        snapshot: &TemperatureTimestampSnapshot,
    ) {
        if let Some(odo) = snapshot.odo {
            self.current_odo = Some(odo);
        }
        if let Some(soc) = snapshot.soc {
            self.current_soc = Some(soc);
        }
        if let Some(temp) = snapshot.temp {
            self.current_temp = Some(temp);
        }
    }
}
