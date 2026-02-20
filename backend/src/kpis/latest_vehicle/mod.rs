mod postgres;
mod row_mapper;
mod sqlite;

pub(crate) use postgres::fetch_latest_vehicle_kpis_postgres;
pub(crate) use sqlite::fetch_latest_vehicle_kpis_sqlite;
