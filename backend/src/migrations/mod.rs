mod postgres;
mod sqlite;

pub(crate) const SQLITE_MIGRATION_0001: &str =
    include_str!("../../migrations/sqlite/0001_init.sql");
pub(crate) const SQLITE_MIGRATIONS: &[(&str, &str)] = &[("0001_init", SQLITE_MIGRATION_0001)];
#[cfg(test)]
pub(crate) const LEGACY_SQLITE_SCHEMA: &str = include_str!("../../schema.sql");
pub(crate) const POSTGRES_MIGRATION_0001: &str =
    include_str!("../../migrations/postgres/0001_init.sql");
pub(crate) const POSTGRES_MIGRATIONS: &[(&str, &str)] = &[("0001_init", POSTGRES_MIGRATION_0001)];

pub(crate) use postgres::apply_postgres_schema;
pub(crate) use sqlite::apply_schema;
