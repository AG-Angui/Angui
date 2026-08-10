use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0044_add_learning_category_governance"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0044_add_learning_category_governance.sql"),
                include_str!("../sql/postgres/up/0044_add_learning_category_governance.sql"),
                include_str!("../sql/mysql/up/0044_add_learning_category_governance.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[
                (
                    "learning categories or resource category assignments exist",
                    "SELECT 1 FROM learning_categories LIMIT 1",
                ),
                (
                    "learning resources use a governed category",
                    "SELECT 1 FROM learning_resources WHERE category_id IS NOT NULL LIMIT 1",
                ),
            ],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0044_remove_learning_category_governance.sql"),
                include_str!("../sql/postgres/down/0044_remove_learning_category_governance.sql"),
                include_str!("../sql/mysql/down/0044_remove_learning_category_governance.sql"),
            ),
        )
        .await
    }
}
