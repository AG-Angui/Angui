use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0036_add_intake_ai_initial_review"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0036_add_intake_ai_initial_review.sql"),
                include_str!("../sql/postgres/up/0036_add_intake_ai_initial_review.sql"),
                include_str!("../sql/mysql/up/0036_add_intake_ai_initial_review.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[(
                "intake AI initial-review results or confirmation states exist",
                "SELECT 1 FROM intake_sessions WHERE ai_initial_review_status <> 'not_started' OR status IN ('awaiting_family_review', 'ready_for_second_confirmation') LIMIT 1",
            )],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0036_remove_intake_ai_initial_review.sql"),
                include_str!("../sql/postgres/down/0036_remove_intake_ai_initial_review.sql"),
                include_str!("../sql/mysql/down/0036_remove_intake_ai_initial_review.sql"),
            ),
        )
        .await
    }
}
