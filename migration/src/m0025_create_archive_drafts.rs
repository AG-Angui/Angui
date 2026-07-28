use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0025_create_archive_drafts"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0025_create_archive_drafts.sql"),
                include_str!("../sql/postgres/up/0025_create_archive_drafts.sql"),
                include_str!("../sql/mysql/up/0025_create_archive_drafts.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[(
                "archive drafts exist",
                "SELECT 1 FROM archive_drafts LIMIT 1",
            )],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0025_drop_archive_drafts.sql"),
                include_str!("../sql/postgres/down/0025_drop_archive_drafts.sql"),
                include_str!("../sql/mysql/down/0025_drop_archive_drafts.sql"),
            ),
        )
        .await
    }
}
