use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0037_add_ai_review_materials"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0037_add_ai_review_materials.sql"),
                include_str!("../sql/postgres/up/0037_add_ai_review_materials.sql"),
                include_str!("../sql/mysql/up/0037_add_ai_review_materials.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[
                (
                    "AI profile candidates exist",
                    "SELECT 1 FROM intake_profile_drafts LIMIT 1",
                ),
                (
                    "controlled source records exist",
                    "SELECT 1 FROM case_source_records LIMIT 1",
                ),
                (
                    "archive review material exists",
                    "SELECT 1 FROM archive_review_materials LIMIT 1",
                ),
            ],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0037_remove_ai_review_materials.sql"),
                include_str!("../sql/postgres/down/0037_remove_ai_review_materials.sql"),
                include_str!("../sql/mysql/down/0037_remove_ai_review_materials.sql"),
            ),
        )
        .await
    }
}
