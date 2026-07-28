use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0026_add_locked_user_status"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0026_add_locked_user_status.sql"),
                include_str!("../sql/postgres/up/0026_add_locked_user_status.sql"),
                include_str!("../sql/mysql/up/0026_add_locked_user_status.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[(
                "locked users exist",
                "SELECT 1 FROM users WHERE status = 'locked' LIMIT 1",
            )],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0026_remove_locked_user_status.sql"),
                include_str!("../sql/postgres/down/0026_remove_locked_user_status.sql"),
                include_str!("../sql/mysql/down/0026_remove_locked_user_status.sql"),
            ),
        )
        .await
    }
}
