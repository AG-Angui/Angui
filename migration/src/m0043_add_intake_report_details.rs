use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0043_add_intake_report_details"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0043_add_intake_report_details.sql"),
                include_str!("../sql/postgres/up/0043_add_intake_report_details.sql"),
                include_str!("../sql/mysql/up/0043_add_intake_report_details.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[(
                "intake report photos exist",
                "SELECT 1 FROM intake_session_photos LIMIT 1",
            )],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0043_remove_intake_report_details.sql"),
                include_str!("../sql/postgres/down/0043_remove_intake_report_details.sql"),
                include_str!("../sql/mysql/down/0043_remove_intake_report_details.sql"),
            ),
        )
        .await
    }
}
