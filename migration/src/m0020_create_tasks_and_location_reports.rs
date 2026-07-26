use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0020_create_tasks_and_location_reports"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0020_create_tasks_and_location_reports.sql"),
                include_str!("../sql/postgres/up/0020_create_tasks_and_location_reports.sql"),
                include_str!("../sql/mysql/up/0020_create_tasks_and_location_reports.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[
                (
                    "task location reports exist",
                    "SELECT 1 FROM task_location_reports LIMIT 1",
                ),
                (
                    "task assignments exist",
                    "SELECT 1 FROM task_assignments LIMIT 1",
                ),
                ("tasks exist", "SELECT 1 FROM tasks LIMIT 1"),
            ],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0020_drop_tasks_and_location_reports.sql"),
                include_str!("../sql/postgres/down/0020_drop_tasks_and_location_reports.sql"),
                include_str!("../sql/mysql/down/0020_drop_tasks_and_location_reports.sql"),
            ),
        )
        .await
    }
}
