use sea_orm_migration::prelude::*;

use crate::{execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0066_extend_knowledge_schema"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0066_extend_knowledge_schema.sql"),
                include_str!("../sql/postgres/up/0066_extend_knowledge_schema.sql"),
                include_str!("../sql/mysql/up/0066_extend_knowledge_schema.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let metadata_check = if manager.get_database_backend()
            == sea_orm_migration::sea_orm::DbBackend::MySql
        {
            "SELECT id FROM knowledge_import_batches WHERE CAST(source_metadata_json AS CHAR) <> '{}' LIMIT 1"
        } else {
            "SELECT id FROM knowledge_import_batches WHERE CAST(source_metadata_json AS TEXT) <> '{}' LIMIT 1"
        };
        crate::ensure_rollback_is_safe(manager, &[
            ("external experience identifiers would be lost", "SELECT id FROM knowledge_items WHERE external_id IS NOT NULL LIMIT 1"),
            ("category hierarchy or descriptions would be lost", "SELECT id FROM knowledge_terms WHERE parent_id IS NOT NULL OR description <> '' LIMIT 1"),
            ("source file metadata would be lost", metadata_check),
        ]).await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0066_extend_knowledge_schema.sql"),
                include_str!("../sql/postgres/down/0066_extend_knowledge_schema.sql"),
                include_str!("../sql/mysql/down/0066_extend_knowledge_schema.sql"),
            ),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::{Migrator, MigratorTrait};
    use sea_orm_migration::sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

    #[tokio::test]
    async fn legacy_knowledge_defaults_survive_upgrade_and_empty_extension_rollback() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, Some(65)).await.unwrap();
        db.execute_unprepared("INSERT INTO knowledge_import_batches (id, knowledge_base_id, file_name, status, total_rows, valid_rows, invalid_rows, created_by_user_id, created_at, updated_at) VALUES ('batch', 'learning-materials', 'legacy.csv', 'previewed', 0, 0, 0, 'system', '2026-01-01', '2026-01-01')").await.unwrap();
        Migrator::up(&db, Some(1)).await.unwrap();
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT source_metadata_json FROM knowledge_import_batches WHERE id = 'batch'",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            row.try_get::<String>("", "source_metadata_json").unwrap(),
            "{}"
        );
        db.execute_unprepared("UPDATE knowledge_import_batches SET source_metadata_json = '{\"version\":\"1.0\"}' WHERE id = 'batch'").await.unwrap();
        assert!(Migrator::down(&db, Some(1)).await.is_err());
        db.execute_unprepared(
            "UPDATE knowledge_import_batches SET source_metadata_json = '{}' WHERE id = 'batch'",
        )
        .await
        .unwrap();
        Migrator::down(&db, Some(1)).await.unwrap();
        Migrator::up(&db, Some(1)).await.unwrap();
    }
}
