use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0027_add_archive_review_lifecycle"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0027_add_archive_review_lifecycle.sql"),
                include_str!("../sql/postgres/up/0027_add_archive_review_lifecycle.sql"),
                include_str!("../sql/mysql/up/0027_add_archive_review_lifecycle.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[ (
                "reviewed archive drafts exist",
                "SELECT 1 FROM archive_drafts WHERE status <> 'draft' OR deidentification_status <> 'manual_review_required' OR deidentified_by_user_id IS NOT NULL OR deidentified_at IS NOT NULL OR deidentification_reason IS NOT NULL OR reviewed_by_user_id IS NOT NULL OR reviewed_at IS NOT NULL OR review_reason IS NOT NULL OR version <> 1 OR usage_scope <> 'internal_archive' OR retention_status <> 'retained' LIMIT 1",
            ) ],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0027_remove_archive_review_lifecycle.sql"),
                include_str!("../sql/postgres/down/0027_remove_archive_review_lifecycle.sql"),
                include_str!("../sql/mysql/down/0027_remove_archive_review_lifecycle.sql"),
            ),
        )
        .await
    }
}
