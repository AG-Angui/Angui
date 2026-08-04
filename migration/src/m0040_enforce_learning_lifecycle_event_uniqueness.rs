use crate::{execute_script, sql_for_backend};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0040_enforce_learning_lifecycle_event_uniqueness"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        const INDEX_NAME: &str = "uq_learning_content_review_event";
        if !manager
            .has_index("learning_content_review_events", INDEX_NAME)
            .await?
        {
            execute_script(
                manager,
                sql_for_backend(
                    manager,
                    include_str!(
                        "../sql/sqlite/up/0040_enforce_learning_lifecycle_event_uniqueness.sql"
                    ),
                    include_str!(
                        "../sql/postgres/up/0040_enforce_learning_lifecycle_event_uniqueness.sql"
                    ),
                    include_str!(
                        "../sql/mysql/up/0040_enforce_learning_lifecycle_event_uniqueness.sql"
                    ),
                ),
            )
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!(
                    "../sql/sqlite/down/0040_remove_learning_lifecycle_event_uniqueness.sql"
                ),
                include_str!(
                    "../sql/postgres/down/0040_remove_learning_lifecycle_event_uniqueness.sql"
                ),
                include_str!(
                    "../sql/mysql/down/0040_remove_learning_lifecycle_event_uniqueness.sql"
                ),
            ),
        )
        .await
    }
}
