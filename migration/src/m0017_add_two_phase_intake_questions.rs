use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

const V2_SEED_INTEGRITY_QUERY: &str = r#"
SELECT 1
FROM intake_question_definitions
WHERE id IN ('intake-q-0201', 'intake-q-0202', 'intake-q-0203', 'intake-q-0204', 'intake-q-0205', 'intake-q-0206', 'intake-q-0207', 'intake-q-0208', 'intake-q-0209')
  AND NOT (
    (id = 'intake-q-0201' AND version = 2 AND field_code = 'basic_information' AND prompt = 'Please describe the person using information your family can verify.' AND display_order = 1 AND is_required = TRUE AND max_answer_chars = 500 AND status = 'active' AND created_at = '2026-07-25T00:00:00.000Z' AND updated_at = '2026-07-25T00:00:00.000Z') OR
    (id = 'intake-q-0202' AND version = 2 AND field_code = 'health_status' AND prompt = 'What health, cognitive, mobility, or medication concerns should be recorded as unconfirmed draft information?' AND display_order = 2 AND is_required = FALSE AND max_answer_chars = 1000 AND status = 'active' AND created_at = '2026-07-25T00:00:00.000Z' AND updated_at = '2026-07-25T00:00:00.000Z') OR
    (id = 'intake-q-0203' AND version = 2 AND field_code = 'behavior_habits' AND prompt = 'What routines, preferences, or behaviors may help verify future leads?' AND display_order = 3 AND is_required = FALSE AND max_answer_chars = 800 AND status = 'active' AND created_at = '2026-07-25T00:00:00.000Z' AND updated_at = '2026-07-25T00:00:00.000Z') OR
    (id = 'intake-q-0204' AND version = 2 AND field_code = 'last_seen' AND prompt = 'When and where was the person last seen? Include uncertainty in time, place, transport, or companions.' AND display_order = 4 AND is_required = TRUE AND max_answer_chars = 1000 AND status = 'active' AND created_at = '2026-07-25T00:00:00.000Z' AND updated_at = '2026-07-25T00:00:00.000Z') OR
    (id = 'intake-q-0205' AND version = 2 AND field_code = 'frequent_locations' AND prompt = 'Which places do they commonly visit? Please avoid unrelated private addresses.' AND display_order = 5 AND is_required = FALSE AND max_answer_chars = 800 AND status = 'active' AND created_at = '2026-07-25T00:00:00.000Z' AND updated_at = '2026-07-25T00:00:00.000Z') OR
    (id = 'intake-q-0206' AND version = 2 AND field_code = 'suspicious_motive' AND prompt = 'Are there any possible reasons, plans, or concerns that need careful human follow-up? Mark unknown when unsure.' AND display_order = 6 AND is_required = FALSE AND max_answer_chars = 800 AND status = 'active' AND created_at = '2026-07-25T00:00:00.000Z' AND updated_at = '2026-07-25T00:00:00.000Z') OR
    (id = 'intake-q-0207' AND version = 2 AND field_code = 'belongings' AND prompt = 'What clothing, bags, phone, identification, or other belongings were they carrying?' AND display_order = 7 AND is_required = FALSE AND max_answer_chars = 800 AND status = 'active' AND created_at = '2026-07-25T00:00:00.000Z' AND updated_at = '2026-07-25T00:00:00.000Z') OR
    (id = 'intake-q-0208' AND version = 2 AND field_code = 'transport_ability' AND prompt = 'How might they travel independently? Include walking, vehicle, public transport, and companion uncertainty.' AND display_order = 8 AND is_required = FALSE AND max_answer_chars = 600 AND status = 'active' AND created_at = '2026-07-25T00:00:00.000Z' AND updated_at = '2026-07-25T00:00:00.000Z') OR
    (id = 'intake-q-0209' AND version = 2 AND field_code = 'follow_up_clues' AND prompt = 'Is there later information or a lead that still needs human verification?' AND display_order = 9 AND is_required = FALSE AND max_answer_chars = 1000 AND status = 'active' AND created_at = '2026-07-25T00:00:00.000Z' AND updated_at = '2026-07-25T00:00:00.000Z')
  )
LIMIT 1
"#;

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
                    V2_SEED_INTEGRITY_QUERY,
                ),
                (
                    "the migration status backup is incomplete",
                    "SELECT 1 WHERE (SELECT COUNT(*) FROM intake_question_definition_status_backup) <> 8",
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
