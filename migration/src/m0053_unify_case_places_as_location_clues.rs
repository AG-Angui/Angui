use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::Statement;

use crate::{execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0053_unify_case_places_as_location_clues"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0053_unify_case_places_as_location_clues.sql"),
                include_str!("../sql/postgres/up/0053_unify_case_places_as_location_clues.sql"),
                include_str!("../sql/mysql/up/0053_unify_case_places_as_location_clues.sql"),
            ),
        )
        .await?;
        let mismatch = manager
            .get_connection()
            .query_one_raw(Statement::from_string(
                manager.get_database_backend(),
                "SELECT p.id FROM case_places p LEFT JOIN clues c ON c.legacy_case_place_id = p.id LEFT JOIN audit_events a ON a.entity_id = c.id AND a.action = 'location_clue.migrated' WHERE c.id IS NULL OR a.id IS NULL OR LENGTH(c.id) > 36 OR c.case_id <> p.case_id OR c.created_by_user_id <> p.created_by_user_id OR c.visibility <> p.visibility OR c.source <> p.source OR c.raw_record_reference <> p.id OR c.location_text <> p.address OR COALESCE(c.longitude, -999) <> COALESCE(p.longitude, -999) OR COALESCE(c.latitude, -999) <> COALESCE(p.latitude, -999) OR c.status <> CASE p.review_status WHEN 'confirmed' THEN 'confirmed' WHEN 'rejected' THEN 'rejected' ELSE 'pending_review' END LIMIT 1".to_owned(),
            ))
            .await?;
        if let Some(row) = mismatch {
            let id: String = row.try_get("", "id")?;
            return Err(DbErr::Custom(format!(
                "location clue migration mismatch for case_places.id={id}"
            )));
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0053_unify_case_places_as_location_clues.sql"),
                include_str!("../sql/postgres/down/0053_unify_case_places_as_location_clues.sql"),
                include_str!("../sql/mysql/down/0053_unify_case_places_as_location_clues.sql"),
            ),
        )
        .await
    }
}
