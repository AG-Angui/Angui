use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0042_create_ai_executions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0042_create_ai_executions.sql"),
                include_str!("../sql/postgres/up/0042_create_ai_executions.sql"),
                include_str!("../sql/mysql/up/0042_create_ai_executions.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[(
                "AI execution records exist",
                "SELECT 1 FROM ai_executions LIMIT 1",
            )],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0042_drop_ai_executions.sql"),
                include_str!("../sql/postgres/down/0042_drop_ai_executions.sql"),
                include_str!("../sql/mysql/down/0042_drop_ai_executions.sql"),
            ),
        )
        .await
    }
}
