use crate::RankingsQuery;

/// Builds the rankings page SQL, including optional filters in bind order.
pub(super) fn build_rankings_page_sql(params: &RankingsQuery) -> String {
    let mut sql = String::from(
        r#"
        SELECT
          r.rank_position,
          r.vehicle_uid,
          r.score,
          r.confidence_level,
          r.cohort_key,
          r.cohort_size,
          r.sample_gate_passed
        FROM cohort_ranking_snapshot r
        JOIN vehicle v ON v.vehicle_uid = r.vehicle_uid
        WHERE r.ranking_type = ?
          AND r.timeframe = ?
          AND r.temperature_bin = ?
          AND r.computed_at = ?
        "#,
    );

    append_optional_filters(&mut sql, params);
    sql.push_str(" ORDER BY r.rank_position ASC LIMIT ? OFFSET ? ");
    sql
}

/// Appends optional ranking filters in the same bind order used by query binds.
fn append_optional_filters(sql: &mut String, params: &RankingsQuery) {
    if params.make.is_some() {
        sql.push_str(" AND COALESCE(v.make, 'unknown') = ? ");
    }
    if params.model.is_some() {
        sql.push_str(" AND COALESCE(v.model, 'unknown') = ? ");
    }
    if params.trim.is_some() {
        sql.push_str(" AND COALESCE(v.trim, 'unknown') = ? ");
    }
    if params.powertrain_class.is_some() {
        sql.push_str(" AND COALESCE(v.powertrain_class, 'unknown') = ? ");
    }
}
