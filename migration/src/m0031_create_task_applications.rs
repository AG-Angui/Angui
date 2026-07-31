use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0031_create_task_applications"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0031_create_task_applications.sql"),
                include_str!("../sql/postgres/up/0031_create_task_applications.sql"),
                include_str!("../sql/mysql/up/0031_create_task_applications.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[(
                "task applications exist",
                "SELECT 1 FROM task_applications LIMIT 1",
            )],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0031_drop_task_applications.sql"),
                include_str!("../sql/postgres/down/0031_drop_task_applications.sql"),
                include_str!("../sql/mysql/down/0031_drop_task_applications.sql"),
            ),
        )
        .await
    }
}
