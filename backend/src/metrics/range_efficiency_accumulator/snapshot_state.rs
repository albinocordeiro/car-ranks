use super::{RangeEfficiencyAccumulator, RangeEfficiencySnapshot};

impl RangeEfficiencyAccumulator {
    /// Applies sparse snapshot fields onto the rolling state for the stream.
    ///
    /// The raw feed omits unchanged values, so this method carries forward the
    /// last known odometer, SOC, and speed until new observations arrive.
    pub(super) fn refresh_latest_snapshot_state(
        &mut self,
        ts_seconds: i64,
        snapshot: &RangeEfficiencySnapshot,
    ) {
        if let Some(odo) = snapshot.odo {
            self.current_odo = Some(odo);
        }
        if let Some(soc) = snapshot.soc {
            self.current_soc = Some(soc);
        }
        if let Some(speed) = snapshot.speed {
            self.current_speed = Some(speed);
        }
        if snapshot.regen_power_kw.is_some() || snapshot.traction_power_kw.is_some() {
            self.power_windows.push((
                ts_seconds,
                snapshot.regen_power_kw,
                snapshot.traction_power_kw,
            ));
        }

        // Expose the freshest SOC reading for downstream KPI builders.
        self.latest_soc = self.current_soc;
    }
}
