use sea_orm_migration::sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};

use crate::{Migrator, MigratorTrait};

async fn legacy_database() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    Migrator::up(&db, Some(64)).await.unwrap();
    db.execute_unprepared(
        "INSERT INTO users (id, email, display_name, account_type, password_hash, status, created_at, updated_at) VALUES ('location-user', 'location-user@demo.invalid', 'Synthetic', 'member', 'hash', 'active', '2026-01-01', '2026-01-01');
         INSERT INTO cases (id, case_code, status, created_at, updated_at) VALUES ('location-case', 'AG-LOCATION', 'active', '2026-01-01', '2026-01-01'), ('other-case', 'AG-OTHER', 'active', '2026-01-01', '2026-01-01');
         INSERT INTO clues (id, case_id, status, source, content, location_text, location_precision, longitude, latitude, created_at, updated_at)
         VALUES ('first', 'location-case', 'confirmed', 'manual', 'Synthetic observation', 'Synthetic public square', 'exact', 120.0, 30.0, '2026-01-01', '2026-01-01'),
                ('second', 'location-case', 'pending_review', 'manual', 'A separate observation at the same point', 'Synthetic public square', 'exact', 120.0, 30.0, '2026-01-01', '2026-01-01'),
                ('no-location', 'location-case', 'pending_review', 'manual', 'A non-spatial observation', NULL, NULL, NULL, NULL, '2026-01-01', '2026-01-01'),
                ('coordinates-only', 'location-case', 'pending_review', 'manual', 'Coordinate-only observation', NULL, NULL, 121.0, 31.0, '2026-01-01', '2026-01-01');
         INSERT INTO tasks (id, case_id, title, objective, area_text, latitude, longitude, due_at, background, risk_level, risk_notes, safety_briefing, expected_feedback, status, created_by_user_id, created_at, updated_at)
         VALUES ('synthetic-task', 'location-case', 'Task', 'Check public area', 'Synthetic public square', 30.0, 120.0, '2026-01-02', '', 'low', '', '', '', 'pending_claim', 'location-user', '2026-01-01', '2026-01-01');
         INSERT INTO locations (id, case_id, location_text, location_precision, created_at, updated_at) VALUES ('existing', 'other-case', 'Existing independent location', 'address', '2026-01-01', '2026-01-01');
         INSERT INTO locations (id, case_id, location_text, status, merged_into_id, created_at, updated_at) VALUES ('existing-merged', 'other-case', 'Existing merged location', 'merged', 'existing', '2026-01-01', '2026-01-01');",
    ).await.unwrap();
    db
}

async fn scalar(db: &DatabaseConnection, query: &str, column: &str) -> String {
    db.query_one_raw(Statement::from_string(DbBackend::Sqlite, query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", column)
        .unwrap()
}

#[tokio::test]
async fn backfill_preserves_distinct_observations_and_existing_locations() {
    let db = legacy_database().await;
    Migrator::up(&db, None).await.unwrap();
    assert_eq!(
        scalar(
            &db,
            "SELECT location_id AS value FROM clues WHERE id = 'first'",
            "value"
        )
        .await,
        "clue:first"
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT location_id AS value FROM clues WHERE id = 'second'",
            "value"
        )
        .await,
        "clue:second"
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT COALESCE(location_id, 'NULL') AS value FROM clues WHERE id = 'no-location'",
            "value"
        )
        .await,
        "NULL"
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT location_text AS value FROM locations WHERE id = 'clue:coordinates-only'",
            "value"
        )
        .await,
        ""
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT location_precision AS value FROM locations WHERE id = 'clue:first'",
            "value"
        )
        .await,
        "exact"
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT location_id AS value FROM tasks WHERE id = 'synthetic-task'",
            "value"
        )
        .await,
        "task:synthetic-task"
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT merged_into_id AS value FROM locations WHERE id = 'existing-merged'",
            "value"
        )
        .await,
        "existing"
    );
    let violations = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA foreign_key_check",
        ))
        .await
        .unwrap();
    assert!(violations.is_empty());
}

#[tokio::test]
async fn location_references_reject_cross_case_updates_and_missing_locations() {
    let db = legacy_database().await;
    Migrator::up(&db, None).await.unwrap();
    for query in [
        "UPDATE clues SET location_id = 'existing' WHERE id = 'first'",
        "UPDATE tasks SET location_id = 'existing' WHERE id = 'synthetic-task'",
        "UPDATE clues SET location_id = 'missing' WHERE id = 'first'",
        "UPDATE locations SET case_id = 'other-case' WHERE id = 'clue:first'",
        "UPDATE clues SET case_id = 'other-case' WHERE id = 'first'",
        "INSERT INTO clues (id, case_id, status, source, content, location_id, created_at, updated_at) VALUES ('bad', 'location-case', 'pending_review', 'manual', 'Synthetic', 'existing', '2026-01-01', '2026-01-01')",
    ] {
        assert!(
            db.execute_unprepared(query).await.is_err(),
            "must reject {query}"
        );
    }
    // Sharing is explicit and only permitted within the same case.
    db.execute_unprepared(
        "UPDATE tasks SET location_id = 'clue:first' WHERE id = 'synthetic-task'",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn rollback_restores_canonical_edits_and_can_be_upgraded_again() {
    let db = legacy_database().await;
    Migrator::up(&db, None).await.unwrap();
    db.execute_unprepared("UPDATE locations SET location_text = 'Corrected synthetic square', longitude = 120.5 WHERE id = 'clue:first'; UPDATE locations SET location_text = 'Corrected task area' WHERE id = 'task:synthetic-task'").await.unwrap();
    Migrator::down(&db, Some(1)).await.unwrap();
    assert_eq!(
        scalar(
            &db,
            "SELECT location_text AS value FROM clues WHERE id = 'first'",
            "value"
        )
        .await,
        "Corrected synthetic square"
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT area_text AS value FROM tasks WHERE id = 'synthetic-task'",
            "value"
        )
        .await,
        "Corrected task area"
    );
    assert_eq!(scalar(&db, "SELECT COALESCE(location_text, 'NULL') AS value FROM clues WHERE id = 'coordinates-only'", "value").await, "NULL");
    assert_eq!(
        scalar(
            &db,
            "SELECT merged_into_id AS value FROM locations WHERE id = 'existing-merged'",
            "value"
        )
        .await,
        "existing"
    );
    Migrator::up(&db, None).await.unwrap();
    assert_eq!(
        scalar(
            &db,
            "SELECT location_text AS value FROM locations WHERE id = 'clue:first'",
            "value"
        )
        .await,
        "Corrected synthetic square"
    );
}

#[tokio::test]
async fn rollback_blocks_loss_of_location_lifecycle_information_before_ddl() {
    let db = legacy_database().await;
    Migrator::up(&db, None).await.unwrap();
    db.execute_unprepared("UPDATE locations SET status = 'retired' WHERE id = 'clue:first'")
        .await
        .unwrap();
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    assert_eq!(
        scalar(
            &db,
            "SELECT location_id AS value FROM clues WHERE id = 'first'",
            "value"
        )
        .await,
        "clue:first"
    );
}

#[tokio::test]
async fn rollback_blocks_new_independent_locations_before_earlier_table_drop() {
    let db = legacy_database().await;
    Migrator::up(&db, None).await.unwrap();
    db.execute_unprepared("INSERT INTO locations (id, case_id, location_text, location_precision, created_at, updated_at) VALUES ('new-location', 'location-case', 'A new independent synthetic location', 'exact', '2026-02-01', '2026-02-01')").await.unwrap();
    let error = Migrator::down(&db, None).await.unwrap_err();
    assert!(error.to_string().contains("new independent locations"));
    assert_eq!(
        scalar(
            &db,
            "SELECT location_text AS value FROM locations WHERE id = 'new-location'",
            "value"
        )
        .await,
        "A new independent synthetic location"
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT location_id AS value FROM clues WHERE id = 'first'",
            "value"
        )
        .await,
        "clue:first"
    );
}

#[tokio::test]
async fn invalid_legacy_coordinates_fail_before_schema_changes() {
    let db = legacy_database().await;
    db.execute_unprepared("UPDATE clues SET longitude = 181 WHERE id = 'first'")
        .await
        .unwrap();
    let error = Migrator::up(&db, None).await.unwrap_err();
    assert!(error.to_string().contains("legacy clue has invalid"));
    assert!(
        db.execute_unprepared("SELECT location_id FROM clues")
            .await
            .is_err()
    );
    // The constraint remains the m0064 constraint, confirming no DDL ran.
    assert!(db.execute_unprepared("INSERT INTO locations (id, case_id, location_text, location_precision, created_at, updated_at) VALUES ('invalid-precision', 'location-case', 'Synthetic', 'exact', '2026-01-01', '2026-01-01')").await.is_err());
}
