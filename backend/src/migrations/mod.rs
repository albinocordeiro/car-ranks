mod postgres;

pub(crate) const POSTGRES_MIGRATION_0001: &str =
    include_str!("../../migrations/postgres/0001_init.sql");
pub(crate) const POSTGRES_MIGRATION_0002: &str =
    include_str!("../../migrations/postgres/0002_auth_ownership.sql");
pub(crate) const POSTGRES_MIGRATION_0003: &str =
    include_str!("../../migrations/postgres/0003_internal_job_runs.sql");
pub(crate) const POSTGRES_MIGRATION_0004: &str =
    include_str!("../../migrations/postgres/0004_internal_job_locks.sql");
pub(crate) const POSTGRES_MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", POSTGRES_MIGRATION_0001),
    ("0002_auth_ownership", POSTGRES_MIGRATION_0002),
    ("0003_internal_job_runs", POSTGRES_MIGRATION_0003),
    ("0004_internal_job_locks", POSTGRES_MIGRATION_0004),
];

pub(crate) use postgres::apply_postgres_schema;
