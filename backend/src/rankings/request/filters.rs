use std::collections::BTreeMap;

use crate::RankingsQuery;

/// Materializes the response filter map from query parameters.
pub(in crate::rankings) fn build_rankings_filters(
    params: &RankingsQuery,
) -> BTreeMap<String, Option<String>> {
    let mut filters = BTreeMap::new();
    filters.insert(
        "powertrain_class".to_string(),
        Some(
            params
                .powertrain_class
                .clone()
                .unwrap_or_else(|| "bev".to_string()),
        ),
    );
    filters.insert("make".to_string(), params.make.clone());
    filters.insert("model".to_string(), params.model.clone());
    filters.insert("trim".to_string(), params.trim.clone());
    filters.insert("year_band".to_string(), params.year_band.clone());
    filters.insert("region".to_string(), params.region.clone());
    filters
}
