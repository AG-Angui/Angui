use crate::{execute_script, sql_for_backend};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0050_add_intake_primary_photo.sql"),
                include_str!("../sql/postgres/up/0050_add_intake_primary_photo.sql"),
                include_str!("../sql/mysql/up/0050_add_intake_primary_photo.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0050_remove_intake_primary_photo.sql"),
                include_str!("../sql/postgres/down/0050_remove_intake_primary_photo.sql"),
                include_str!("../sql/mysql/down/0050_remove_intake_primary_photo.sql"),
            ),
        )
        .await
    }
}
