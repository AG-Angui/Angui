use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, DbBackend, TransactionTrait};

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

const MYSQL_CLUE_LIFECYCLE_COLUMNS: &[(&str, &str)] = &[
    (
        "source_type",
        "source_type VARCHAR(32) NOT NULL DEFAULT 'manual_report'",
    ),
    ("raw_record_reference", "raw_record_reference VARCHAR(500)"),
    ("reported_at", "reported_at VARCHAR(40)"),
    ("confirmed_at", "confirmed_at VARCHAR(40)"),
    ("location_precision", "location_precision VARCHAR(32)"),
    ("next_action", "next_action VARCHAR(500)"),
    (
        "linked_task_reference",
        "linked_task_reference VARCHAR(120)",
    ),
    ("related_clue_id", "related_clue_id VARCHAR(36)"),
    ("relationship_type", "relationship_type VARCHAR(32)"),
    ("review_reason", "review_reason VARCHAR(1000)"),
];

const MYSQL_CLUE_ATTACHMENT_LINKS_TABLE: &str = r#"
CREATE TABLE clue_attachment_links (
    clue_id VARCHAR(36) NOT NULL,
    attachment_id VARCHAR(36) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    PRIMARY KEY (clue_id, attachment_id),
    CONSTRAINT fk_clue_attachment_links_clue FOREIGN KEY (clue_id) REFERENCES clues(id) ON DELETE CASCADE,
    CONSTRAINT fk_clue_attachment_links_attachment FOREIGN KEY (attachment_id) REFERENCES case_attachments(id) ON DELETE CASCADE
) ENGINE=InnoDB
"#;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0019_expand_clue_lifecycle"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::MySql {
            return up_mysql_resumably(manager).await;
        }

        let sql = sql_for_backend(
            manager,
            include_str!("../sql/sqlite/up/0019_expand_clue_lifecycle.sql"),
            include_str!("../sql/postgres/up/0019_expand_clue_lifecycle.sql"),
            include_str!("../sql/mysql/up/0019_expand_clue_lifecycle.sql"),
        );
        execute_script(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // These columns preserve source and review provenance. A rollback is
        // allowed only before the new lifecycle data has been used.
        match manager.get_database_backend() {
            DbBackend::MySql => down_mysql_with_write_lock(manager).await,
            DbBackend::Postgres => down_postgres_with_write_lock(manager).await,
            DbBackend::Sqlite => down_sqlite_in_transaction(manager).await,
        }
    }
}

async fn up_mysql_resumably(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    // MySQL auto-commits ALTER TABLE. Query the catalog before constructing one
    // combined ALTER so a retry after a partial execution adds only what remains.
    // The fixed, migration-owned definitions mean the generated SQL has no input
    // from callers.
    let mut missing_columns = Vec::with_capacity(MYSQL_CLUE_LIFECYCLE_COLUMNS.len());
    for (name, definition) in MYSQL_CLUE_LIFECYCLE_COLUMNS {
        if !manager.has_column("clues", name).await? {
            missing_columns.push(*definition);
        }
    }
    if !missing_columns.is_empty() {
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE clues {}",
                missing_columns
                    .iter()
                    .map(|definition| format!("ADD COLUMN {definition}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .await?;
    }

    manager
        .get_connection()
        .execute_unprepared("UPDATE clues SET reported_at = created_at WHERE reported_at IS NULL")
        .await?;
    manager
        .get_connection()
        .execute_unprepared("ALTER TABLE clues MODIFY reported_at VARCHAR(40) NOT NULL")
        .await?;

    if !manager
        .has_index("clues", "idx_clues_related_clue_id")
        .await?
    {
        manager
            .get_connection()
            .execute_unprepared("CREATE INDEX idx_clues_related_clue_id ON clues(related_clue_id)")
            .await?;
    }

    if !manager.has_table("clue_attachment_links").await? {
        manager
            .get_connection()
            .execute_unprepared(MYSQL_CLUE_ATTACHMENT_LINKS_TABLE)
            .await?;
    }
    if !manager
        .has_index(
            "clue_attachment_links",
            "idx_clue_attachment_links_attachment_id",
        )
        .await?
    {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_clue_attachment_links_attachment_id ON clue_attachment_links(attachment_id)",
            )
            .await?;
    }

    Ok(())
}

fn rollback_safety_checks() -> [(&'static str, &'static str); 2] {
    [
        (
            "clue attachment links exist",
            "SELECT 1 FROM clue_attachment_links LIMIT 1",
        ),
        (
            "clue lifecycle provenance exists",
            "SELECT 1 FROM clues WHERE source_type <> 'manual_report' OR raw_record_reference IS NOT NULL OR confirmed_at IS NOT NULL OR location_precision IS NOT NULL OR next_action IS NOT NULL OR linked_task_reference IS NOT NULL OR related_clue_id IS NOT NULL OR relationship_type IS NOT NULL OR review_reason IS NOT NULL LIMIT 1",
        ),
    ]
}

async fn execute_down(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let sql = sql_for_backend(
        manager,
        include_str!("../sql/sqlite/down/0019_expand_clue_lifecycle.sql"),
        include_str!("../sql/postgres/down/0019_expand_clue_lifecycle.sql"),
        include_str!("../sql/mysql/down/0019_expand_clue_lifecycle.sql"),
    );
    execute_script(manager, sql).await
}

async fn down_postgres_with_write_lock(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let transaction = manager.get_connection().begin().await?;
    let locked_manager = SchemaManager::new(&transaction);
    locked_manager
        .get_connection()
        .execute_unprepared("LOCK TABLE clues, clue_attachment_links IN ACCESS EXCLUSIVE MODE")
        .await?;
    ensure_rollback_is_safe(&locked_manager, &rollback_safety_checks()).await?;
    execute_down(&locked_manager).await?;
    transaction.commit().await
}

async fn down_sqlite_in_transaction(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    // Keeping the preflight and DDL on one SQLite connection ensures a writer
    // that races the preflight cannot commit while this rollback reaches DDL.
    // If an earlier writer holds the required lock, the down migration fails
    // rather than silently dropping its data.
    let transaction = manager.get_connection().begin().await?;
    let locked_manager = SchemaManager::new(&transaction);
    ensure_rollback_is_safe(&locked_manager, &rollback_safety_checks()).await?;
    execute_down(&locked_manager).await?;
    transaction.commit().await
}

async fn down_mysql_with_write_lock(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    // LOCK TABLES is connection-scoped, so use a DatabaseTransaction to keep a
    // single MySQL connection pinned from the safety checks through every DDL
    // statement. The locks block both inserts into clues/link rows and updates
    // to the lifecycle columns until UNLOCK TABLES runs.
    let transaction = manager.get_connection().begin().await?;
    let locked_manager = SchemaManager::new(&transaction);
    locked_manager
        .get_connection()
        .execute_unprepared(
            "LOCK TABLES clues WRITE, clue_attachment_links WRITE, case_attachments READ",
        )
        .await?;

    let rollback_result = async {
        ensure_rollback_is_safe(&locked_manager, &rollback_safety_checks()).await?;
        execute_down(&locked_manager).await
    }
    .await;
    let unlock_result = locked_manager
        .get_connection()
        .execute_unprepared("UNLOCK TABLES")
        .await;

    match (rollback_result, unlock_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(_)) => transaction.commit().await,
    }
}
