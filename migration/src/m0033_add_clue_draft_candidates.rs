use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0033_add_clue_draft_candidates"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0033_add_clue_draft_candidates.sql"),
                include_str!("../sql/postgres/up/0033_add_clue_draft_candidates.sql"),
                include_str!("../sql/mysql/up/0033_add_clue_draft_candidates.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[(
                "clue draft candidates or reviews exist",
                "SELECT 1 FROM clue_drafts WHERE candidate_json <> '{}' OR review_status <> 'pending_review' LIMIT 1",
            )],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0033_remove_clue_draft_candidates.sql"),
                include_str!("../sql/postgres/down/0033_remove_clue_draft_candidates.sql"),
                include_str!("../sql/mysql/down/0033_remove_clue_draft_candidates.sql"),
            ),
        )
        .await
    }
}
