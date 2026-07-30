pub use sea_orm_migration::prelude::*;

mod m0001_create_cases;
mod m0002_create_elder_profiles;
mod m0003_create_clues;
mod m0004_create_audit_events;
mod m0005_create_users;
mod m0006_create_auth_sessions;
mod m0007_create_case_memberships;
mod m0008_create_clue_attributions;
mod m0009_add_learner_role;
mod m0010_create_intake_sessions;
mod m0011_create_intake_question_definitions;
mod m0012_split_account_type_and_capabilities;
mod m0013_create_intake_session_answers;
mod m0014_confirm_intake_sessions;
mod m0015_create_intake_prompt_templates;
mod m0016_add_intake_assessments;
mod m0017_add_two_phase_intake_questions;
mod m0018_create_case_places_and_attachments;
mod m0019_expand_clue_lifecycle;
mod m0020_create_tasks_and_location_reports;
mod m0021_create_user_profiles_and_elder_profile_revisions;
mod m0022_create_summary_drafts;
mod m0023_create_clue_drafts;
mod m0024_enforce_single_published_summary_draft;
mod m0025_create_archive_drafts;
mod m0026_add_locked_user_status;
mod m0027_add_archive_review_lifecycle;
mod m0028_create_task_operation_idempotency;

use sea_orm_migration::sea_orm::{DbBackend, Statement};

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m0001_create_cases::Migration),
            Box::new(m0002_create_elder_profiles::Migration),
            Box::new(m0003_create_clues::Migration),
            Box::new(m0004_create_audit_events::Migration),
            Box::new(m0005_create_users::Migration),
            Box::new(m0006_create_auth_sessions::Migration),
            Box::new(m0007_create_case_memberships::Migration),
            Box::new(m0008_create_clue_attributions::Migration),
            Box::new(m0009_add_learner_role::Migration),
            Box::new(m0010_create_intake_sessions::Migration),
            Box::new(m0011_create_intake_question_definitions::Migration),
            Box::new(m0012_split_account_type_and_capabilities::Migration),
            Box::new(m0013_create_intake_session_answers::Migration),
            Box::new(m0014_confirm_intake_sessions::Migration),
            Box::new(m0015_create_intake_prompt_templates::Migration),
            Box::new(m0016_add_intake_assessments::Migration),
            Box::new(m0017_add_two_phase_intake_questions::Migration),
            Box::new(m0018_create_case_places_and_attachments::Migration),
            Box::new(m0019_expand_clue_lifecycle::Migration),
            Box::new(m0020_create_tasks_and_location_reports::Migration),
            Box::new(m0021_create_user_profiles_and_elder_profile_revisions::Migration),
            Box::new(m0022_create_summary_drafts::Migration),
            Box::new(m0023_create_clue_drafts::Migration),
            Box::new(m0024_enforce_single_published_summary_draft::Migration),
            Box::new(m0025_create_archive_drafts::Migration),
            Box::new(m0026_add_locked_user_status::Migration),
            Box::new(m0027_add_archive_review_lifecycle::Migration),
            Box::new(m0028_create_task_operation_idempotency::Migration),
        ]
    }
}

pub(crate) fn sql_for_backend(
    manager: &SchemaManager<'_>,
    sqlite: &'static str,
    postgres: &'static str,
    mysql: &'static str,
) -> &'static str {
    match manager.get_database_backend() {
        DbBackend::Sqlite => sqlite,
        DbBackend::Postgres => postgres,
        DbBackend::MySql => mysql,
    }
}

pub(crate) async fn execute_script(manager: &SchemaManager<'_>, script: &str) -> Result<(), DbErr> {
    let statements: Vec<_> = script
        .split("-- statement-break")
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
        .collect();

    let restores_sqlite_foreign_keys = manager.get_database_backend() == DbBackend::Sqlite
        && script
            .to_ascii_lowercase()
            .contains("pragma foreign_keys = off");

    for (index, statement) in statements.iter().enumerate() {
        // InnoDB rejects dropping an index that backs a foreign key (error 1553).
        // A later DROP TABLE removes that table's indexes and constraints itself,
        // so skip only that provably redundant index drop and keep all others strict.
        if manager.get_database_backend() == DbBackend::MySql
            && mysql_drop_index_is_redundant(statement, &statements[index + 1..])
        {
            continue;
        }

        if let Err(error) = manager.get_connection().execute_unprepared(statement).await {
            if restores_sqlite_foreign_keys {
                let _ = manager
                    .get_connection()
                    .execute_unprepared("PRAGMA foreign_keys = ON;")
                    .await;
            }
            return Err(error);
        }
    }

    Ok(())
}

/// Refuses a rollback before it discards data written after the migration ran.
///
/// Each query must return at least one row when rollback is unsafe. Keeping this
/// check in Rust makes the invariant consistent across SQLite, PostgreSQL, and
/// MySQL without relying on dialect-specific SQL error tricks.
pub(crate) async fn ensure_rollback_is_safe(
    manager: &SchemaManager<'_>,
    checks: &[(&str, &str)],
) -> Result<(), DbErr> {
    for (description, query) in checks {
        let found = manager
            .get_connection()
            .query_one(Statement::from_string(
                manager.get_database_backend(),
                (*query).to_owned(),
            ))
            .await?;
        if found.is_some() {
            return Err(DbErr::Custom(format!(
                "destructive rollback blocked: {description}; archive or remove the data explicitly before retrying"
            )));
        }
    }

    Ok(())
}

fn mysql_drop_index_is_redundant(statement: &str, later_statements: &[&str]) -> bool {
    let Some(indexed_table) = mysql_drop_index_table(statement) else {
        return false;
    };

    later_statements.iter().any(|later_statement| {
        mysql_drop_table(later_statement).as_deref() == Some(indexed_table.as_str())
    })
}

fn mysql_drop_index_table(statement: &str) -> Option<String> {
    let normalized = normalize_mysql_statement(statement);
    let drop_index = normalized.strip_prefix("drop index ")?;
    let (_, table) = drop_index.rsplit_once(" on ")?;
    normalize_mysql_identifier(table)
}

fn mysql_drop_table(statement: &str) -> Option<String> {
    let normalized = normalize_mysql_statement(statement);
    let table = normalized
        .strip_prefix("drop table if exists ")
        .or_else(|| normalized.strip_prefix("drop table "))?;
    normalize_mysql_identifier(table)
}

fn normalize_mysql_statement(statement: &str) -> String {
    statement
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_ascii_lowercase()
}

fn normalize_mysql_identifier(identifier: &str) -> Option<String> {
    let identifier = identifier.trim().trim_matches('`').trim_matches('"').trim();
    (!identifier.is_empty()).then(|| identifier.to_owned())
}

#[cfg(test)]
mod tests {
    use sea_orm_migration::sea_orm::{ConnectionTrait, Database, Statement};

    use super::{Migrator, MigratorTrait, mysql_drop_index_is_redundant};

    const MYSQL_DOWN_SCRIPTS: &[&str] = &[
        include_str!("../sql/mysql/down/0001_drop_cases.sql"),
        include_str!("../sql/mysql/down/0002_drop_elder_profiles.sql"),
        include_str!("../sql/mysql/down/0003_drop_clues.sql"),
        include_str!("../sql/mysql/down/0004_drop_audit_events.sql"),
        include_str!("../sql/mysql/down/0005_drop_users.sql"),
        include_str!("../sql/mysql/down/0006_drop_auth_sessions.sql"),
        include_str!("../sql/mysql/down/0007_drop_case_memberships.sql"),
        include_str!("../sql/mysql/down/0008_drop_clue_attributions.sql"),
        include_str!("../sql/mysql/down/0009_remove_learner_role.sql"),
        include_str!("../sql/mysql/down/0010_drop_intake_sessions.sql"),
        include_str!("../sql/mysql/down/0011_drop_intake_question_definitions.sql"),
        include_str!("../sql/mysql/down/0012_restore_single_user_role.sql"),
    ];

    const ROLE_CONSTRAINT_SCRIPTS: &[(&str, &str)] = &[
        (
            "sqlite account types",
            include_str!("../sql/sqlite/up/0012_split_account_type_and_capabilities.sql"),
        ),
        (
            "sqlite global capabilities",
            include_str!("../sql/sqlite/up/0012_split_account_type_and_capabilities.sql"),
        ),
        (
            "sqlite case memberships",
            include_str!("../sql/sqlite/up/0007_create_case_memberships.sql"),
        ),
        (
            "postgres account types",
            include_str!("../sql/postgres/up/0012_split_account_type_and_capabilities.sql"),
        ),
        (
            "postgres global capabilities",
            include_str!("../sql/postgres/up/0012_split_account_type_and_capabilities.sql"),
        ),
        (
            "postgres case memberships",
            include_str!("../sql/postgres/up/0007_create_case_memberships.sql"),
        ),
        (
            "mysql account types",
            include_str!("../sql/mysql/up/0012_split_account_type_and_capabilities.sql"),
        ),
        (
            "mysql global capabilities",
            include_str!("../sql/mysql/up/0012_split_account_type_and_capabilities.sql"),
        ),
        (
            "mysql case memberships",
            include_str!("../sql/mysql/up/0007_create_case_memberships.sql"),
        ),
    ];

    const SUMMARY_DRAFT_SCRIPTS: &[(&str, &str)] = &[
        (
            "sqlite",
            include_str!("../sql/sqlite/up/0022_create_summary_drafts.sql"),
        ),
        (
            "postgres",
            include_str!("../sql/postgres/up/0022_create_summary_drafts.sql"),
        ),
        (
            "mysql",
            include_str!("../sql/mysql/up/0022_create_summary_drafts.sql"),
        ),
    ];

    const PUBLISHED_SUMMARY_DRAFT_CONSTRAINT_SCRIPTS: &[(&str, &str)] = &[
        (
            "sqlite",
            include_str!("../sql/sqlite/up/0024_enforce_single_published_summary_draft.sql"),
        ),
        (
            "postgres",
            include_str!("../sql/postgres/up/0024_enforce_single_published_summary_draft.sql"),
        ),
        (
            "mysql",
            include_str!("../sql/mysql/up/0024_enforce_single_published_summary_draft.sql"),
        ),
    ];

    const ARCHIVE_DRAFT_SCRIPTS: &[(&str, &str)] = &[
        (
            "sqlite",
            include_str!("../sql/sqlite/up/0025_create_archive_drafts.sql"),
        ),
        (
            "postgres",
            include_str!("../sql/postgres/up/0025_create_archive_drafts.sql"),
        ),
        (
            "mysql",
            include_str!("../sql/mysql/up/0025_create_archive_drafts.sql"),
        ),
    ];

    const ARCHIVE_REVIEW_LIFECYCLE_UP_SCRIPTS: &[(&str, &str)] = &[
        (
            "sqlite",
            include_str!("../sql/sqlite/up/0027_add_archive_review_lifecycle.sql"),
        ),
        (
            "postgres",
            include_str!("../sql/postgres/up/0027_add_archive_review_lifecycle.sql"),
        ),
        (
            "mysql",
            include_str!("../sql/mysql/up/0027_add_archive_review_lifecycle.sql"),
        ),
    ];

    const ARCHIVE_REVIEW_LIFECYCLE_DOWN_SCRIPTS: &[(&str, &str)] = &[
        (
            "sqlite",
            include_str!("../sql/sqlite/down/0027_remove_archive_review_lifecycle.sql"),
        ),
        (
            "postgres",
            include_str!("../sql/postgres/down/0027_remove_archive_review_lifecycle.sql"),
        ),
        (
            "mysql",
            include_str!("../sql/mysql/down/0027_remove_archive_review_lifecycle.sql"),
        ),
    ];

    const LOCKED_USER_STATUS_SCRIPTS: &[(&str, &str)] = &[
        (
            "sqlite",
            include_str!("../sql/sqlite/up/0026_add_locked_user_status.sql"),
        ),
        (
            "postgres",
            include_str!("../sql/postgres/up/0026_add_locked_user_status.sql"),
        ),
        (
            "mysql",
            include_str!("../sql/mysql/up/0026_add_locked_user_status.sql"),
        ),
    ];

    const LOCKED_USER_STATUS_DOWN_SCRIPTS: &[(&str, &str)] = &[
        (
            "sqlite",
            include_str!("../sql/sqlite/down/0026_remove_locked_user_status.sql"),
        ),
        (
            "postgres",
            include_str!("../sql/postgres/down/0026_remove_locked_user_status.sql"),
        ),
        (
            "mysql",
            include_str!("../sql/mysql/down/0026_remove_locked_user_status.sql"),
        ),
    ];

    #[tokio::test]
    async fn sqlite_migrations_support_up_down_up() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");

        Migrator::up(&database, None)
            .await
            .expect("all up migrations should succeed");
        Migrator::down(&database, None)
            .await
            .expect("all down migrations should succeed");
        Migrator::up(&database, None)
            .await
            .expect("migrations should be repeatable after rollback");
    }

    #[tokio::test]
    async fn sqlite_archive_draft_migration_preserves_data_and_refuses_unsafe_rollback() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, Some(24))
            .await
            .expect("schema before archive draft migration should succeed");
        insert_member_and_session(&database, "archive-draft").await;
        database
            .execute_unprepared(
                "INSERT INTO cases (id, case_code, status, created_at, updated_at) VALUES ('archive-draft-case', 'AG-00000025', 'resolved', '2026-07-28T00:00:00.000Z', '2026-07-28T00:00:00.000Z')",
            )
            .await
            .expect("archive draft case fixture should be stored");

        Migrator::up(&database, Some(1))
            .await
            .expect("archive draft migration should succeed");
        database
            .execute_unprepared(
                "INSERT INTO archive_drafts (id, case_id, status, content, source_scope_json, deidentification_status, template_version, provider_model, created_by_user_id, created_at, updated_at) VALUES ('archive-draft-1', 'archive-draft-case', 'draft', 'Internal test archive draft.', '[\"confirmed_clue_metadata\"]', 'manual_review_required', 'v1', NULL, 'archive-draft-user', '2026-07-28T00:00:00.000Z', '2026-07-28T00:00:00.000Z')",
            )
            .await
            .expect("archive draft should satisfy schema constraints");
        assert!(Migrator::down(&database, Some(1)).await.is_err());
        assert!(
            database
                .query_one(Statement::from_string(
                    sea_orm_migration::sea_orm::DbBackend::Sqlite,
                    "SELECT 1 FROM archive_drafts WHERE id = 'archive-draft-1'",
                ))
                .await
                .expect("archive draft query should succeed")
                .is_some()
        );
    }

    #[tokio::test]
    async fn sqlite_archive_review_lifecycle_preserves_drafts_and_refuses_reviewed_rollback() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, Some(26))
            .await
            .expect("schema before archive review lifecycle should succeed");
        insert_member_and_session(&database, "archive-review").await;
        database
            .execute_unprepared(
                "INSERT INTO cases (id, case_code, status, created_at, updated_at) VALUES ('archive-review-case', 'AG-00000027', 'resolved', '2026-07-29T00:00:00.000Z', '2026-07-29T00:00:00.000Z'); INSERT INTO archive_drafts (id, case_id, status, content, source_scope_json, deidentification_status, template_version, provider_model, created_by_user_id, created_at, updated_at) VALUES ('archive-review-draft', 'archive-review-case', 'draft', 'Internal test archive draft.', '[\"confirmed_clue_metadata\"]', 'manual_review_required', 'v1', NULL, 'archive-review-user', '2026-07-29T00:00:00.000Z', '2026-07-29T00:00:00.000Z')",
            )
            .await
            .expect("pre-lifecycle archive draft should be stored");

        Migrator::up(&database, Some(1))
            .await
            .expect("archive review lifecycle migration should succeed");
        assert!(
            database
                .execute_unprepared(
                    "UPDATE archive_drafts SET status = 'published' WHERE id = 'archive-review-draft'",
                )
                .await
                .is_err(),
            "database constraints must reject publication before de-identification and controlled usage are recorded"
        );
        database
            .execute_unprepared(
                "UPDATE archive_drafts SET status = 'pending_review', deidentification_status = 'deidentified', deidentified_by_user_id = 'archive-review-user', deidentified_at = '2026-07-29T00:01:00.000Z', deidentification_reason = 'manual confirmation', version = 2 WHERE id = 'archive-review-draft'",
            )
            .await
            .expect("review lifecycle fields should accept a de-identified draft");
        assert!(Migrator::down(&database, Some(1)).await.is_err());
        assert!(
            database
                .query_one(Statement::from_string(
                    sea_orm_migration::sea_orm::DbBackend::Sqlite,
                    "SELECT 1 FROM archive_drafts WHERE id = 'archive-review-draft' AND status = 'pending_review' AND version = 2",
                ))
                .await
                .expect("reviewed archive draft query should succeed")
                .is_some(),
            "a refused rollback must preserve reviewed archive draft data"
        );
    }

    #[tokio::test]
    async fn sqlite_locked_user_status_migration_enforces_and_preserves_locked_accounts() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, Some(25))
            .await
            .expect("schema before locked status migration should succeed");
        insert_member_and_session(&database, "locked-account").await;
        assert!(
            database
                .execute_unprepared(
                    "UPDATE users SET status = 'locked' WHERE id = 'locked-account-user'",
                )
                .await
                .is_err()
        );

        Migrator::up(&database, Some(1))
            .await
            .expect("locked status migration should succeed");
        database
            .execute_unprepared(
                "UPDATE users SET status = 'locked' WHERE id = 'locked-account-user'",
            )
            .await
            .expect("locked status should be accepted after migration");
        assert!(Migrator::down(&database, Some(1)).await.is_err());
        let locked = database
            .query_one(Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT 1 FROM users WHERE id = 'locked-account-user' AND status = 'locked'",
            ))
            .await
            .expect("locked account query should succeed");
        assert!(locked.is_some());
    }

    #[tokio::test]
    async fn sqlite_published_summary_draft_constraint_deduplicates_and_enforces_one_per_case() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, Some(23))
            .await
            .expect("schema before published summary constraint should succeed");
        insert_member_and_session(&database, "published-summary").await;
        database
            .execute_unprepared(
                "INSERT INTO cases (id, case_code, status, created_at, updated_at) VALUES ('published-summary-case', 'AG-00000024', 'active', '2026-07-28T00:00:00.000Z', '2026-07-28T00:00:00.000Z'); INSERT INTO summary_drafts (id, case_id, status, content, source_scope_json, template_version, provider_model, publication_eligible, generated_by_user_id, reviewed_by_user_id, reviewed_at, review_reason, created_at, updated_at) VALUES ('published-summary-older', 'published-summary-case', 'published', 'Older controlled summary.', '{}', 'v1', NULL, 1, 'published-summary-user', NULL, NULL, NULL, '2026-07-28T00:00:00.000Z', '2026-07-28T00:00:00.000Z'), ('published-summary-newer', 'published-summary-case', 'published', 'Newer controlled summary.', '{}', 'v1', NULL, 1, 'published-summary-user', NULL, NULL, NULL, '2026-07-28T00:00:01.000Z', '2026-07-28T00:00:01.000Z')",
            )
            .await
            .expect("legacy duplicate published summaries should be insertable before the constraint");

        Migrator::up(&database, Some(1))
            .await
            .expect("published summary constraint migration should succeed");
        let published = database
            .query_one(Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT id FROM summary_drafts WHERE case_id = 'published-summary-case' AND status = 'published'",
            ))
            .await
            .expect("published summary query should succeed")
            .expect("one published summary should remain after migration");
        assert_eq!(
            published.try_get::<String>("", "id").unwrap(),
            "published-summary-newer"
        );
        assert!(database
            .execute_unprepared(
                "INSERT INTO summary_drafts (id, case_id, status, content, source_scope_json, template_version, provider_model, publication_eligible, generated_by_user_id, reviewed_by_user_id, reviewed_at, review_reason, created_at, updated_at) VALUES ('published-summary-conflict', 'published-summary-case', 'published', 'Conflicting controlled summary.', '{}', 'v1', NULL, 1, 'published-summary-user', NULL, NULL, NULL, '2026-07-28T00:00:02.000Z', '2026-07-28T00:00:02.000Z')",
            )
            .await
            .is_err());

        Migrator::down(&database, Some(1))
            .await
            .expect("removing the published summary constraint should not discard data");
        database
            .execute_unprepared(
                "INSERT INTO summary_drafts (id, case_id, status, content, source_scope_json, template_version, provider_model, publication_eligible, generated_by_user_id, reviewed_by_user_id, reviewed_at, review_reason, created_at, updated_at) VALUES ('published-summary-after-down', 'published-summary-case', 'published', 'Rollback verification summary.', '{}', 'v1', NULL, 1, 'published-summary-user', NULL, NULL, NULL, '2026-07-28T00:00:03.000Z', '2026-07-28T00:00:03.000Z')",
            )
            .await
            .expect("rollback should remove only the constraint");
    }

    #[tokio::test]
    async fn sqlite_rollbacks_refuse_to_discard_post_migration_data() {
        let confirmation_database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&confirmation_database, Some(14))
            .await
            .expect("schema through confirmation migration should succeed");
        insert_member_and_session(&confirmation_database, "confirmation").await;
        confirmation_database
            .execute_unprepared(
                "UPDATE intake_sessions SET confirmed_by_user_id = 'confirmation-user', confirmed_at = '2026-07-25T01:00:00.000Z' WHERE id = 'confirmation-session'",
            )
            .await
            .expect("confirmation data should be stored");
        assert!(
            Migrator::down(&confirmation_database, Some(1))
                .await
                .is_err()
        );

        let template_database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&template_database, Some(15))
            .await
            .expect("schema through prompt template migration should succeed");
        template_database
            .execute_unprepared(
                "INSERT INTO ai_prompt_templates (id, purpose, version, system_instruction, status, created_by_user_id, published_by_user_id, published_at, created_at, updated_at) VALUES ('operator-template', 'case_summary_draft', 'operator-v1', 'Operator-managed configuration.', 'draft', NULL, NULL, NULL, '2026-07-25T01:00:00.000Z', '2026-07-25T01:00:00.000Z')",
            )
            .await
            .expect("operator template should be stored");
        assert!(Migrator::down(&template_database, Some(1)).await.is_err());
        assert!(
            template_database
                .query_one(Statement::from_string(
                    sea_orm_migration::sea_orm::DbBackend::Sqlite,
                    "SELECT 1 FROM ai_prompt_templates WHERE id = 'operator-template'",
                ))
                .await
                .expect("template query should succeed")
                .is_some()
        );

        let assessment_database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&assessment_database, Some(16))
            .await
            .expect("schema through assessment migration should succeed");
        insert_member_and_session(&assessment_database, "assessment").await;
        assessment_database
            .execute_unprepared(
                "UPDATE intake_sessions SET assessment_json = '[{\"kind\":\"risk\"}]' WHERE id = 'assessment-session'",
            )
            .await
            .expect("assessment data should be stored");
        assert!(Migrator::down(&assessment_database, Some(1)).await.is_err());
    }

    #[tokio::test]
    async fn sqlite_lifecycle_rollback_refuses_to_discard_reported_at() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, None)
            .await
            .expect("all migrations should succeed");
        database
            .execute_unprepared(
                "INSERT INTO cases (id, case_code, status, created_at, updated_at) VALUES ('reported-at-case', 'AG-00000019', 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'); INSERT INTO clues (id, case_id, status, source, content, reported_at, created_at, updated_at) VALUES ('reported-at-clue', 'reported-at-case', 'pending_review', 'family', 'Reported after the original event.', '2026-07-26T12:30:00.000Z', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z')",
            )
            .await
            .expect("clue with a distinct report time should be stored");

        // Roll back every migration after the lifecycle migration before
        // exercising migration 0019's reported-at safety guard.
        let migrations_after_lifecycle =
            u32::try_from(Migrator::migrations().len() - 18).expect("migration count must fit u32");
        assert!(
            Migrator::down(&database, Some(migrations_after_lifecycle))
                .await
                .is_err()
        );
        assert!(database
            .query_one(Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT 1 FROM clues WHERE id = 'reported-at-clue' AND reported_at = '2026-07-26T12:30:00.000Z'",
            ))
            .await
            .expect("report-time query should succeed")
            .is_some());
    }

    #[tokio::test]
    async fn sqlite_task_migration_enforces_assignment_location_and_rollback_integrity() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, None)
            .await
            .expect("all migrations should succeed");
        database
            .execute_unprepared(
                "INSERT INTO users (id, email, display_name, account_type, password_hash, status, created_at, updated_at) VALUES ('task-commander', 'task-commander@demo.invalid', 'Task commander', 'member', 'hash', 'active', '2026-07-26T00:00:00.000Z', '2026-07-26T00:00:00.000Z'), ('task-volunteer', 'task-volunteer@demo.invalid', 'Task volunteer', 'member', 'hash', 'active', '2026-07-26T00:00:00.000Z', '2026-07-26T00:00:00.000Z'); INSERT INTO cases (id, case_code, status, created_at, updated_at) VALUES ('task-case', 'AG-00000020', 'active', '2026-07-26T00:00:00.000Z', '2026-07-26T00:00:00.000Z')",
            )
            .await
            .expect("task fixtures should be stored");

        assert!(
            database
                .execute_unprepared(&task_insert_sql("invalid-task", "missing-case"))
                .await
                .is_err()
        );
        assert!(database
            .execute_unprepared(
                "INSERT INTO tasks (id, case_id, source_clue_id, title, objective, area_text, latitude, longitude, due_at, background, risk_level, risk_notes, safety_briefing, expected_feedback, status, result_summary, created_by_user_id, created_at, updated_at) VALUES ('incomplete-coordinate-task', 'task-case', NULL, 'Verify north gate', 'Check the reported route', 'North gate to market', 31.2, NULL, '2026-07-27T12:00:00.000Z', 'A reviewed report needs field verification.', 'medium', 'Stay in public areas.', 'Keep contact and stop if conditions change.', 'Submit a text report for commander review.', 'assigned', NULL, 'task-commander', '2026-07-26T00:00:00.000Z', '2026-07-26T00:00:00.000Z')",
            )
            .await
            .is_err());

        database
            .execute_unprepared(&task_insert_sql("task-1", "task-case"))
            .await
            .expect("task should reference its case");
        assert!(database
            .execute_unprepared(
                "INSERT INTO task_assignments (task_id, volunteer_user_id, assigned_by_user_id, assigned_at, updated_at) VALUES ('task-1', 'missing-volunteer', 'task-commander', '2026-07-26T00:00:00.000Z', '2026-07-26T00:00:00.000Z')",
            )
            .await
            .is_err());
        database
            .execute_unprepared(
                "INSERT INTO task_assignments (task_id, volunteer_user_id, assigned_by_user_id, assigned_at, updated_at) VALUES ('task-1', 'task-volunteer', 'task-commander', '2026-07-26T00:00:00.000Z', '2026-07-26T00:00:00.000Z')",
            )
            .await
            .expect("task should have one assigned volunteer");

        assert!(database
            .execute_unprepared(
                "INSERT INTO task_location_reports (id, task_id, volunteer_user_id, source, latitude, longitude, accuracy_meters, captured_at, retention_expires_at, created_at) VALUES ('invalid-source', 'task-1', 'task-volunteer', 'device', 31.2, 121.5, 20, '2026-07-26T00:10:00.000Z', '2026-07-27T00:10:00.000Z', '2026-07-26T00:10:00.000Z')",
            )
            .await
            .is_err());
        assert!(database
            .execute_unprepared(
                "INSERT INTO task_location_reports (id, task_id, volunteer_user_id, source, latitude, longitude, accuracy_meters, captured_at, retention_expires_at, created_at) VALUES ('invalid-coordinate', 'task-1', 'task-volunteer', 'simulated', 91, 121.5, 20, '2026-07-26T00:10:00.000Z', '2026-07-27T00:10:00.000Z', '2026-07-26T00:10:00.000Z')",
            )
            .await
            .is_err());
        database
            .execute_unprepared(
                "INSERT INTO task_location_reports (id, task_id, volunteer_user_id, source, latitude, longitude, accuracy_meters, captured_at, retention_expires_at, created_at) VALUES ('task-location-1', 'task-1', 'task-volunteer', 'simulated', 31.2, 121.5, 20, '2026-07-26T00:10:00.000Z', '2026-07-27T00:10:00.000Z', '2026-07-26T00:10:00.000Z')",
            )
            .await
            .expect("simulated task location should reference the assignment");

        // Roll back every migration after the task migration; its safety guard
        // must reject this fixture.
        let migrations_after_tasks =
            u32::try_from(Migrator::migrations().len() - 19).expect("migration count must fit u32");
        assert!(
            Migrator::down(&database, Some(migrations_after_tasks))
                .await
                .is_err()
        );
        database
            .execute_unprepared("DELETE FROM tasks WHERE id = 'task-1'")
            .await
            .expect("task cleanup should succeed");
        assert!(database
            .query_one(Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT 1 FROM task_assignments WHERE task_id = 'task-1' UNION ALL SELECT 1 FROM task_location_reports WHERE task_id = 'task-1'",
            ))
            .await
            .expect("cascade query should succeed")
            .is_none());
    }

    #[tokio::test]
    async fn sqlite_two_phase_question_migration_preserves_scope_and_refuses_unsafe_rollback() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, Some(16))
            .await
            .expect("schema before two-phase question migration should succeed");
        database
            .execute_unprepared(
                "INSERT INTO intake_question_definitions (id, version, field_code, prompt, display_order, is_required, max_answer_chars, status, created_at, updated_at) VALUES ('operator-question', 99, 'operator_defined', 'Operator-owned question', 1, 0, 100, 'active', '2026-07-25T01:00:00.000Z', '2026-07-25T01:00:00.000Z')",
            )
            .await
            .expect("operator-owned question should be stored");
        Migrator::up(&database, Some(1))
            .await
            .expect("two-phase question migration should succeed");

        let operator_question = database
            .query_one(Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT status FROM intake_question_definitions WHERE id = 'operator-question'",
            ))
            .await
            .expect("operator question query should succeed")
            .expect("operator question should remain");
        assert_eq!(
            operator_question.try_get::<String>("", "status").unwrap(),
            "active"
        );

        insert_member_and_session(&database, "v2").await;
        database
            .execute_unprepared(
                "UPDATE intake_sessions SET question_set_version = 2 WHERE id = 'v2-session'",
            )
            .await
            .expect("version 2 session should be stored");
        assert!(Migrator::down(&database, Some(1)).await.is_err());
        assert!(
            database
                .query_one(Statement::from_string(
                    sea_orm_migration::sea_orm::DbBackend::Sqlite,
                    "SELECT 1 FROM intake_question_definitions WHERE id = 'intake-q-0201'",
                ))
                .await
                .expect("version 2 question query should succeed")
                .is_some()
        );
    }

    #[tokio::test]
    async fn sqlite_two_phase_question_rollback_refuses_manual_question_changes() {
        let v2_database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        // This assertion is specifically about m0017. Keep m0018 out of the
        // migration history so `down(Some(1))` targets the two-phase-question
        // migration rather than merely rolling back the later empty table.
        Migrator::up(&v2_database, Some(17))
            .await
            .expect("migrations through two-phase questions should succeed");
        v2_database
            .execute_unprepared(
                "UPDATE intake_question_definitions SET prompt = 'Operator-edited prompt' WHERE id = 'intake-q-0201'",
            )
            .await
            .expect("operator edit should be stored without changing the migration timestamp");
        assert!(Migrator::down(&v2_database, Some(1)).await.is_err());
        assert!(v2_database
            .query_one(Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT 1 FROM intake_question_definitions WHERE id = 'intake-q-0201' AND prompt = 'Operator-edited prompt'",
            ))
            .await
            .expect("edited version 2 question query should succeed")
            .is_some());

        let v1_database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&v1_database, Some(17))
            .await
            .expect("migrations through two-phase questions should succeed");
        v1_database
            .execute_unprepared(
                "UPDATE intake_question_definitions SET status = 'active' WHERE id = 'intake-q-0001'",
            )
            .await
            .expect("operator status change should be stored");
        assert!(Migrator::down(&v1_database, Some(1)).await.is_err());
        assert!(v1_database
            .query_one(Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT 1 FROM intake_question_definitions WHERE id = 'intake-q-0001' AND status = 'active'",
            ))
            .await
            .expect("operator status query should succeed")
            .is_some());
    }

    async fn insert_member_and_session(database: &impl ConnectionTrait, prefix: &str) {
        let user_id = format!("{prefix}-user");
        let session_id = format!("{prefix}-session");
        database
            .execute_unprepared(&format!(
                "INSERT INTO users (id, email, display_name, account_type, password_hash, status, created_at, updated_at) VALUES ('{user_id}', '{prefix}@demo.invalid', 'Test member', 'member', 'hash', 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'); INSERT INTO intake_sessions (id, created_by_user_id, case_id, question_set_version, status, answers_json, created_at, updated_at) VALUES ('{session_id}', '{user_id}', NULL, 1, 'collecting', '{{}}', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z')"
            ))
            .await
            .expect("test member and intake session should be stored");
    }

    fn task_insert_sql(id: &str, case_id: &str) -> String {
        format!(
            "INSERT INTO tasks (id, case_id, source_clue_id, title, objective, area_text, latitude, longitude, due_at, background, risk_level, risk_notes, safety_briefing, expected_feedback, status, result_summary, created_by_user_id, created_at, updated_at) VALUES ('{id}', '{case_id}', NULL, 'Verify north gate', 'Check the reported route', 'North gate to market', 31.2, 121.5, '2026-07-27T12:00:00.000Z', 'A reviewed report needs field verification.', 'medium', 'Stay in public areas.', 'Keep contact and stop if conditions change.', 'Submit a text report for commander review.', 'assigned', NULL, 'task-commander', '2026-07-26T00:00:00.000Z', '2026-07-26T00:00:00.000Z')"
        )
    }

    #[tokio::test]
    async fn sqlite_account_type_migration_preserves_references_and_maps_legacy_roles() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, Some(8))
            .await
            .expect("schema before account type migration should succeed");
        database
            .execute_unprepared(
                "INSERT INTO users (id, email, display_name, role, password_hash, status, created_at, updated_at) VALUES ('existing-user', 'existing@demo.invalid', 'Existing user', 'family', 'hash', 'active', '2026-07-23T00:00:00Z', '2026-07-23T00:00:00Z'); INSERT INTO users (id, email, display_name, role, password_hash, status, created_at, updated_at) VALUES ('commander-user', 'commander@demo.invalid', 'Commander user', 'commander', 'hash', 'active', '2026-07-23T00:00:00Z', '2026-07-23T00:00:00Z'); INSERT INTO users (id, email, display_name, role, password_hash, status, created_at, updated_at) VALUES ('volunteer-user', 'volunteer@demo.invalid', 'Volunteer user', 'volunteer', 'hash', 'active', '2026-07-23T00:00:00Z', '2026-07-23T00:00:00Z'); INSERT INTO users (id, email, display_name, role, password_hash, status, created_at, updated_at) VALUES ('admin-user', 'admin@demo.invalid', 'Admin user', 'admin', 'hash', 'active', '2026-07-23T00:00:00Z', '2026-07-23T00:00:00Z')",
            )
            .await
            .expect("old schema should accept existing user");

        Migrator::up(&database, Some(1))
            .await
            .expect("learner role migration should succeed");
        database
            .execute_unprepared(
                "INSERT INTO users (id, email, display_name, role, password_hash, status, created_at, updated_at) VALUES ('learner-user', 'learner@demo.invalid', 'Learner user', 'learner', 'hash', 'active', '2026-07-23T00:00:00Z', '2026-07-23T00:00:00Z')",
            )
            .await
            .expect("learner role schema should accept learner user");

        Migrator::up(&database, None)
            .await
            .expect("account type migration should preserve existing data");
        database
            .execute_unprepared(
                "INSERT INTO auth_sessions (id, user_id, token_hash, expires_at, revoked_at, created_at, last_used_at) VALUES ('existing-session', 'existing-user', 'hash', '2026-07-24T00:00:00Z', NULL, '2026-07-23T00:00:00Z', '2026-07-23T00:00:00Z')",
            )
            .await
            .expect("existing user should remain a valid foreign-key target");
        let migrated = database
            .query_all(Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT id, account_type FROM users ORDER BY id",
            ))
            .await
            .expect("migrated account types should be readable");
        assert_eq!(migrated.len(), 5);
        assert_eq!(
            migrated[0].try_get::<String>("", "account_type").unwrap(),
            "member"
        );
        assert_eq!(
            migrated[1].try_get::<String>("", "account_type").unwrap(),
            "member"
        );
        assert_eq!(
            migrated[2].try_get::<String>("", "account_type").unwrap(),
            "member"
        );
        assert_eq!(
            migrated[3].try_get::<String>("", "account_type").unwrap(),
            "learner"
        );
        assert_eq!(
            migrated[4].try_get::<String>("", "account_type").unwrap(),
            "member"
        );
        let capabilities = database
            .query_all(Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT user_id, capability FROM user_global_capabilities ORDER BY user_id",
            ))
            .await
            .expect("migrated capabilities should be readable");
        assert_eq!(capabilities.len(), 3);
        assert_eq!(
            capabilities[0].try_get::<String>("", "user_id").unwrap(),
            "admin-user"
        );
        assert_eq!(
            capabilities[0].try_get::<String>("", "capability").unwrap(),
            "admin"
        );
        assert_eq!(
            capabilities[1].try_get::<String>("", "user_id").unwrap(),
            "commander-user"
        );
        assert_eq!(
            capabilities[1].try_get::<String>("", "capability").unwrap(),
            "commander"
        );
        assert_eq!(
            capabilities[2].try_get::<String>("", "user_id").unwrap(),
            "volunteer-user"
        );
        assert_eq!(
            capabilities[2].try_get::<String>("", "capability").unwrap(),
            "volunteer"
        );
    }

    #[tokio::test]
    async fn sqlite_role_constraints_reject_unknown_account_types_capabilities_and_case_roles() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite connection should succeed");
        Migrator::up(&database, None)
            .await
            .expect("migrations should succeed");

        assert!(database
            .execute_unprepared(
                "INSERT INTO users (id, email, display_name, account_type, password_hash, status, created_at, updated_at) VALUES ('invalid-user', 'invalid@demo.invalid', 'Invalid', 'operator', 'hash', 'active', '2026-07-24T00:00:00Z', '2026-07-24T00:00:00Z')",
            )
            .await
            .is_err());
        database
            .execute_unprepared(
                "INSERT INTO users (id, email, display_name, account_type, password_hash, status, created_at, updated_at) VALUES ('valid-user', 'valid@demo.invalid', 'Valid', 'member', 'hash', 'active', '2026-07-24T00:00:00Z', '2026-07-24T00:00:00Z'); INSERT INTO cases (id, case_code, status, created_at, updated_at) VALUES ('valid-case', 'AG-00000001', 'active', '2026-07-24T00:00:00Z', '2026-07-24T00:00:00Z')",
            )
            .await
            .expect("valid role fixtures should insert");
        assert!(database
            .execute_unprepared(
                "INSERT INTO user_global_capabilities (user_id, capability, created_at) VALUES ('valid-user', 'operator', '2026-07-24T00:00:00Z')",
            )
            .await
            .is_err());
        assert!(database
            .execute_unprepared(
                "INSERT INTO case_memberships (id, case_id, user_id, role, created_by_user_id, created_at) VALUES ('invalid-membership', 'valid-case', 'valid-user', 'admin', NULL, '2026-07-24T00:00:00Z')",
            )
            .await
            .is_err());
    }

    #[test]
    fn every_database_dialect_keeps_closed_account_capability_and_case_role_constraints() {
        for (name, script) in ROLE_CONSTRAINT_SCRIPTS {
            let normalized = script.to_ascii_lowercase();
            assert!(
                normalized.contains("check"),
                "{name} must constrain role values"
            );
            if name.contains("account types") {
                for account_type in ["member", "learner"] {
                    assert!(
                        normalized.contains(account_type),
                        "{name} must permit {account_type}"
                    );
                }
            } else if name.contains("global capabilities") {
                for capability in ["commander", "volunteer", "admin"] {
                    assert!(
                        normalized.contains(capability),
                        "{name} must permit {capability}"
                    );
                }
            } else {
                for role in ["family", "commander", "volunteer"] {
                    assert!(normalized.contains(role), "{name} must permit {role}");
                }
                assert!(
                    !normalized.contains("'learner'"),
                    "{name} must exclude learner"
                );
                assert!(!normalized.contains("'admin'"), "{name} must exclude admin");
            }
        }
    }

    #[test]
    fn summary_drafts_keep_publication_eligibility_boolean_across_dialects() {
        for (name, script) in SUMMARY_DRAFT_SCRIPTS {
            let normalized = script.to_ascii_lowercase();
            assert!(
                normalized.contains("publication_eligible"),
                "{name} summary drafts must persist publication eligibility"
            );
            if *name == "postgres" {
                assert!(
                    normalized.contains("publication_eligible boolean not null"),
                    "postgres must use a native non-null boolean"
                );
            } else {
                assert!(
                    normalized.contains("check (publication_eligible in (0, 1))"),
                    "{name} must constrain publication eligibility to 0 or 1"
                );
            }
        }
    }

    #[test]
    fn archive_drafts_have_matching_restricted_lifecycle_constraints_across_dialects() {
        for (name, script) in ARCHIVE_DRAFT_SCRIPTS {
            let normalized = script.to_ascii_lowercase();
            for required in [
                "archive_drafts",
                "source_scope_json",
                "manual_review_required",
                "foreign key (case_id)",
                "foreign key (created_by_user_id)",
                "idx_archive_drafts_case_created",
            ] {
                assert!(
                    normalized.contains(required),
                    "{name} archive draft schema must declare {required}"
                );
            }
        }
    }

    #[test]
    fn archive_review_lifecycle_is_constrained_and_safely_reversible_across_dialects() {
        for ((name, up_script), (down_name, down_script)) in ARCHIVE_REVIEW_LIFECYCLE_UP_SCRIPTS
            .iter()
            .zip(ARCHIVE_REVIEW_LIFECYCLE_DOWN_SCRIPTS)
        {
            assert_eq!(
                name, down_name,
                "up and down scripts must use the same dialect"
            );
            let up = up_script.to_ascii_lowercase();
            let down = down_script.to_ascii_lowercase();
            for expected in [
                "archive_drafts",
                "pending_review",
                "published",
                "rejected",
                "withdrawn",
                "deidentified",
                "learning_resource",
                "deidentified_by_user_id",
                "reviewed_by_user_id",
                "version",
            ] {
                assert!(
                    up.contains(expected),
                    "{name} up migration must declare {expected}"
                );
            }
            assert!(
                down.contains("status in ('draft')")
                    && down.contains("deidentification_status in ('manual_review_required')"),
                "{name} down migration must restore the original archive draft lifecycle"
            );
            if *name == "sqlite" {
                assert!(
                    up.contains("create table archive_drafts_with_review_lifecycle")
                        && down.contains("create table archive_drafts_without_review_lifecycle"),
                    "sqlite must rebuild archive_drafts for both migration directions"
                );
            } else {
                assert!(
                    up.contains("alter table archive_drafts")
                        && down.contains("alter table archive_drafts"),
                    "{name} must alter the existing archive_drafts table in both directions"
                );
            }
        }
    }

    #[test]
    fn locked_user_status_is_constrained_across_dialects() {
        for ((name, up_script), (down_name, down_script)) in LOCKED_USER_STATUS_SCRIPTS
            .iter()
            .zip(LOCKED_USER_STATUS_DOWN_SCRIPTS)
        {
            assert_eq!(
                name, down_name,
                "up and down scripts must use the same dialect"
            );
            let up = up_script.to_ascii_lowercase();
            let down = down_script.to_ascii_lowercase();
            assert!(
                up.contains("check (status in ('active', 'disabled', 'locked'))"),
                "{name} up migration must constrain users.status to active, disabled, and locked"
            );
            assert!(
                down.contains("check (status in ('active', 'disabled'))"),
                "{name} down migration must restore the active/disabled users.status constraint"
            );
            assert!(
                !down.contains("'locked'"),
                "{name} down migration must remove locked from the users.status constraint"
            );
            if *name == "sqlite" {
                for (script_name, script, table_name) in [
                    ("up", up.as_str(), "users_with_locked_status"),
                    ("down", down.as_str(), "users_without_locked_status"),
                ] {
                    assert!(
                        script.contains(&format!("create table {table_name}")),
                        "sqlite {script_name} migration must rebuild users with its target status constraint"
                    );
                    assert!(
                        script.contains("drop table users") && script.contains("rename to users"),
                        "sqlite {script_name} migration must replace the users table"
                    );
                }
            } else {
                for (script_name, script) in [("up", up.as_str()), ("down", down.as_str())] {
                    assert!(
                        script.contains("alter table users")
                            && script.contains("users_status_check"),
                        "{name} {script_name} migration must replace the named users.status constraint"
                    );
                }
            }
        }
    }

    #[test]
    fn published_summary_drafts_are_constrained_to_one_per_case_across_dialects() {
        for (name, script) in PUBLISHED_SUMMARY_DRAFT_CONSTRAINT_SCRIPTS {
            let normalized = script.to_ascii_lowercase();
            assert!(
                normalized.contains("update summary_drafts"),
                "{name} must reconcile existing duplicate publications before adding the constraint"
            );
            assert!(
                normalized.contains("idx_summary_drafts_one_published_per_case"),
                "{name} must add a per-case published-summary uniqueness constraint"
            );
            if *name == "mysql" {
                assert!(
                    normalized.contains("if(status = 'published', case_id, null)"),
                    "mysql must emulate the partial unique index with a functional key part"
                );
            } else {
                assert!(
                    normalized.contains("where status = 'published'"),
                    "{name} must scope uniqueness to published summaries"
                );
            }
        }
    }

    #[test]
    fn mysql_down_scripts_skip_indexes_removed_by_a_later_table_drop() {
        for script in MYSQL_DOWN_SCRIPTS {
            let statements: Vec<_> = script
                .split("-- statement-break")
                .map(str::trim)
                .filter(|statement| !statement.is_empty())
                .collect();

            for (index, statement) in statements.iter().enumerate() {
                if statement.to_ascii_lowercase().starts_with("drop index ") {
                    assert!(
                        mysql_drop_index_is_redundant(statement, &statements[index + 1..]),
                        "expected redundant MySQL index drop in statement: {statement}"
                    );
                }
            }
        }
    }

    #[test]
    fn mysql_keeps_index_drops_when_the_table_is_not_dropped_later() {
        assert!(!mysql_drop_index_is_redundant(
            "DROP INDEX idx_users_role_status ON users;",
            &["DROP TABLE IF EXISTS cases;"]
        ));
    }

    #[test]
    fn postgres_confirmation_foreign_key_defers_historical_validation() {
        let script = include_str!("../sql/postgres/up/0014_confirm_intake_sessions.sql")
            .to_ascii_lowercase();
        assert!(script.contains("constraint fk_intake_sessions_confirmer"));
        assert!(script.contains("not valid"));
    }
}
