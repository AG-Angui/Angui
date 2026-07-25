use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0016_add_intake_assessments"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0016_add_intake_assessments.sql"),
                include_str!("../sql/postgres/up/0016_add_intake_assessments.sql"),
                include_str!("../sql/mysql/up/0016_add_intake_assessments.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Destructive rollback: assessments and immutable answer revisions must be archived first.
        ensure_rollback_is_safe(
            manager,
            &[
                (
                    "intake answer revisions exist",
                    "SELECT 1 FROM intake_answer_revisions LIMIT 1",
                ),
                (
                    "intake sessions contain assessment or structured answer data",
                    "SELECT 1 FROM intake_sessions WHERE assessment_json <> '[]' OR structured_answers_json <> '{}' LIMIT 1",
                ),
            ],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0016_add_intake_assessments.sql"),
                include_str!("../sql/postgres/down/0016_add_intake_assessments.sql"),
                include_str!("../sql/mysql/down/0016_add_intake_assessments.sql"),
            ),
        )
        .await
    }
}
