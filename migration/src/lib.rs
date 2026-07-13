pub use sea_orm_migration::prelude::*;

mod m0001_create_cases;
mod m0002_create_elder_profiles;
mod m0003_create_clues;
mod m0004_create_audit_events;
mod m0005_create_users;
mod m0006_create_auth_sessions;
mod m0007_create_case_memberships;
mod m0008_create_clue_attributions;

use sea_orm_migration::sea_orm::DbBackend;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m0001_create_cases::Migration),
            Box::new(m0002_create_elder_profiles::Migration),
            Box::new(m0003_create_clues::Migration),
            Box::new(m0004_create_audit_events::Migration),
            Box::new(m0005_create_users::Migration),
            Box::new(m0006_create_auth_sessions::Migration),
            Box::new(m0007_create_case_memberships::Migration),
            Box::new(m0008_create_clue_attributions::Migration),
        ]
    }
}

pub(crate) fn sql_for_backend(
    manager: &SchemaManager<'_>,
    sqlite: &'static str,
    postgres: &'static str,
    mysql: &'static str,
) -> &'static str {
    match manager.get_database_backend() {
        DbBackend::Sqlite => sqlite,
        DbBackend::Postgres => postgres,
        DbBackend::MySql => mysql,
    }
}

pub(crate) async fn execute_script(manager: &SchemaManager<'_>, script: &str) -> Result<(), DbErr> {
    for statement in script
        .split("-- statement-break")
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
    {
        manager
            .get_connection()
            .execute_unprepared(statement)
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use sea_orm_migration::sea_orm::Database;

    use super::{Migrator, MigratorTrait};

    #[tokio::test]
    async fn sqlite_migrations_support_up_down_up() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");

        Migrator::up(&database, None)
            .await
            .expect("all up migrations should succeed");
        Migrator::down(&database, None)
            .await
            .expect("all down migrations should succeed");
        Migrator::up(&database, None)
            .await
            .expect("migrations should be repeatable after rollback");
    }
}
