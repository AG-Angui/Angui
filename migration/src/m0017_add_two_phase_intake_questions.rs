use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0017_add_two_phase_intake_questions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0017_add_two_phase_intake_questions.sql"),
                include_str!("../sql/postgres/up/0017_add_two_phase_intake_questions.sql"),
                include_str!("../sql/mysql/up/0017_add_two_phase_intake_questions.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Destructive rollback: version 2 sessions require their question definitions.
        // Do not overwrite operator changes or delete version 2 questions not created here.
        ensure_rollback_is_safe(
            manager,
            &[
                (
                    "intake sessions use question set version 2",
                    "SELECT 1 FROM intake_sessions WHERE question_set_version = 2 LIMIT 1",
                ),
                (
                    "version 2 contains questions not created by this migration",
                    "SELECT 1 FROM intake_question_definitions WHERE version = 2 AND id NOT IN ('intake-q-0201', 'intake-q-0202', 'intake-q-0203', 'intake-q-0204', 'intake-q-0205', 'intake-q-0206', 'intake-q-0207', 'intake-q-0208', 'intake-q-0209') LIMIT 1",
                ),
                (
                    "migration-owned version 2 questions were modified",
                    "SELECT 1 FROM intake_question_definitions WHERE id IN ('intake-q-0201', 'intake-q-0202', 'intake-q-0203', 'intake-q-0204', 'intake-q-0205', 'intake-q-0206', 'intake-q-0207', 'intake-q-0208', 'intake-q-0209') AND (version <> 2 OR status <> 'active' OR created_at <> '2026-07-25T00:00:00.000Z' OR updated_at <> '2026-07-25T00:00:00.000Z') LIMIT 1",
                ),
                (
                    "version 1 questions were changed after this migration disabled them",
                    "SELECT 1 FROM intake_question_definitions question JOIN intake_question_definition_status_backup backup ON backup.question_id = question.id WHERE question.status <> 'disabled' LIMIT 1",
                ),
                (
                    "the migration status backup is invalid",
                    "SELECT 1 FROM intake_question_definition_status_backup WHERE question_id NOT IN ('intake-q-0001', 'intake-q-0002', 'intake-q-0003', 'intake-q-0004', 'intake-q-0005', 'intake-q-0006', 'intake-q-0007', 'intake-q-0008') OR previous_status NOT IN ('active', 'disabled') LIMIT 1",
                ),
            ],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0017_add_two_phase_intake_questions.sql"),
                include_str!("../sql/postgres/down/0017_add_two_phase_intake_questions.sql"),
                include_str!("../sql/mysql/down/0017_add_two_phase_intake_questions.sql"),
            ),
        )
        .await
    }
}
