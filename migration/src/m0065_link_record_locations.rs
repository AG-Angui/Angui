use crate::{execute_script, sql_for_backend};
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::Statement;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0065_link_record_locations"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Deterministic identifiers make backfill reviewable without fuzzy merging.
        // Refuse a collision rather than associating unrelated existing locations.
        let collision = manager
            .get_connection()
            .query_one_raw(Statement::from_string(
                manager.get_database_backend(),
                "SELECT id FROM locations WHERE id LIKE 'clue:%' OR id LIKE 'task:%' LIMIT 1",
            ))
            .await?;
        if collision.is_some() {
            return Err(DbErr::Custom(
                "location backfill identifier collision; preserve and rename the conflicting location before retrying".to_owned(),
            ));
        }
        // MySQL DDL commits independently, so reject legacy values which the
        // canonical constraints cannot store before altering any table.
        let invalid = manager
            .get_connection()
            .query_one_raw(Statement::from_string(
                manager.get_database_backend(),
                "SELECT id FROM clues WHERE location_precision NOT IN ('unknown','approximate','exact','address','coordinates') OR longitude < -180 OR longitude > 180 OR latitude < -90 OR latitude > 90 OR location_radius_meters < 0 LIMIT 1",
            ))
            .await?;
        if invalid.is_some() {
            return Err(DbErr::Custom(
                "location backfill blocked: a legacy clue has invalid precision, coordinates, or radius; correct the record before retrying".to_owned(),
            ));
        }
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0065_link_record_locations.sql"),
                include_str!("../sql/postgres/up/0065_link_record_locations.sql"),
                include_str!("../sql/mysql/up/0065_link_record_locations.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Legacy task columns cannot represent all locations. Check before any DDL.
        crate::ensure_rollback_is_safe(manager, &[
            ("new independent locations would be lost by the earlier locations-table rollback", "SELECT id FROM locations WHERE id NOT LIKE 'clue:%' AND id NOT LIKE 'task:%' AND id NOT IN (SELECT id FROM m0065_existing_locations) LIMIT 1"),
            ("a task location cannot fit the legacy area or coordinate columns", "SELECT t.id FROM tasks t JOIN locations l ON l.id = t.location_id WHERE LENGTH(l.location_text) > 500 OR (l.latitude IS NULL AND l.longitude IS NOT NULL) OR (l.latitude IS NOT NULL AND l.longitude IS NULL) LIMIT 1"),
            ("a backfilled location has lifecycle or merge information unavailable in legacy records", "SELECT id FROM locations WHERE (id LIKE 'clue:%' OR id LIKE 'task:%') AND (status <> 'active' OR merged_into_id IS NOT NULL) LIMIT 1"),
            ("a location merge still refers to a backfilled location", "SELECT id FROM locations WHERE merged_into_id LIKE 'clue:%' OR merged_into_id LIKE 'task:%' LIMIT 1"),
            ("a backfilled location is now independent of its legacy records", "SELECT l.id FROM locations l WHERE (l.id LIKE 'clue:%' OR l.id LIKE 'task:%') AND NOT EXISTS (SELECT 1 FROM clues c WHERE c.location_id = l.id) AND NOT EXISTS (SELECT 1 FROM tasks t WHERE t.location_id = l.id) LIMIT 1"),
        ]).await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0065_link_record_locations.sql"),
                include_str!("../sql/postgres/down/0065_link_record_locations.sql"),
                include_str!("../sql/mysql/down/0065_link_record_locations.sql"),
            ),
        )
        .await
    }
}

#[cfg(test)]
mod tests;
