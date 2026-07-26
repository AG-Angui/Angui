use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0019_expand_clue_lifecycle"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = sql_for_backend(
            manager,
            include_str!("../sql/sqlite/up/0019_expand_clue_lifecycle.sql"),
            include_str!("../sql/postgres/up/0019_expand_clue_lifecycle.sql"),
            include_str!("../sql/mysql/up/0019_expand_clue_lifecycle.sql"),
        );
        execute_script(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // These columns preserve source and review provenance. A rollback is
        // allowed only before the new lifecycle data has been used.
        ensure_rollback_is_safe(
            manager,
            &[
                ("clue attachment links exist", "SELECT 1 FROM clue_attachment_links LIMIT 1"),
                (
                    "clue lifecycle provenance exists",
                    "SELECT 1 FROM clues WHERE source_type <> 'manual_report' OR raw_record_reference IS NOT NULL OR confirmed_at IS NOT NULL OR location_precision IS NOT NULL OR next_action IS NOT NULL OR linked_task_reference IS NOT NULL OR related_clue_id IS NOT NULL OR relationship_type IS NOT NULL OR review_reason IS NOT NULL LIMIT 1",
                ),
            ],
        )
        .await?;
        let sql = sql_for_backend(
            manager,
            include_str!("../sql/sqlite/down/0019_expand_clue_lifecycle.sql"),
            include_str!("../sql/postgres/down/0019_expand_clue_lifecycle.sql"),
            include_str!("../sql/mysql/down/0019_expand_clue_lifecycle.sql"),
        );
        execute_script(manager, sql).await
    }
}
