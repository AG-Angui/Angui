use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0023_create_clue_drafts"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0023_create_clue_drafts.sql"),
                include_str!("../sql/postgres/up/0023_create_clue_drafts.sql"),
                include_str!("../sql/mysql/up/0023_create_clue_drafts.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[("clue drafts exist", "SELECT 1 FROM clue_drafts LIMIT 1")],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0023_drop_clue_drafts.sql"),
                include_str!("../sql/postgres/down/0023_drop_clue_drafts.sql"),
                include_str!("../sql/mysql/down/0023_drop_clue_drafts.sql"),
            ),
        )
        .await
    }
}
