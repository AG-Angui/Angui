use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0041_add_learning_content_version_lineage"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0041_add_learning_content_version_lineage.sql"),
                include_str!("../sql/postgres/up/0041_add_learning_content_version_lineage.sql"),
                include_str!("../sql/mysql/up/0041_add_learning_content_version_lineage.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[
                (
                    "learning resource version lineage exists",
                    "SELECT 1 FROM learning_resources WHERE previous_version_id IS NOT NULL LIMIT 1",
                ),
                (
                    "learning question version lineage exists",
                    "SELECT 1 FROM learning_questions WHERE previous_version_id IS NOT NULL LIMIT 1",
                ),
            ],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0041_remove_learning_content_version_lineage.sql"),
                include_str!(
                    "../sql/postgres/down/0041_remove_learning_content_version_lineage.sql"
                ),
                include_str!("../sql/mysql/down/0041_remove_learning_content_version_lineage.sql"),
            ),
        )
        .await
    }
}
