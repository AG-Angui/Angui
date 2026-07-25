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

use sea_orm_migration::sea_orm::DbBackend;

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

    for (index, statement) in statements.iter().enumerate() {
        // InnoDB rejects dropping an index that backs a foreign key (error 1553).
        // A later DROP TABLE removes that table's indexes and constraints itself,
        // so skip only that provably redundant index drop and keep all others strict.
        if manager.get_database_backend() == DbBackend::MySql
            && mysql_drop_index_is_redundant(statement, &statements[index + 1..])
        {
            continue;
        }

        manager
            .get_connection()
            .execute_unprepared(statement)
            .await?;
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
}
