use super::RangeEfficiencyAccumulator;

impl RangeEfficiencyAccumulator {
    /// Buckets segment efficiency values for city and highway sub-metrics.
    ///
    /// A single segment can contribute to both collections when thresholds are
    /// changed over time, but with current thresholds these ranges are disjoint.
    pub(super) fn push_speed_segment_efficiency(&mut self, segment_speed: f64, wh_per_km: f64) {
        if segment_speed < 45.0 {
            self.urban_wh_per_km_points.push(wh_per_km);
        }
        if segment_speed >= 80.0 {
            self.highway_wh_per_km_points.push(wh_per_km);
        }
    }
}
