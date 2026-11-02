use crate::{execute_script, sql_for_backend};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0064_create_locations"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0064_create_locations.sql"),
                include_str!("../sql/postgres/up/0064_create_locations.sql"),
                include_str!("../sql/mysql/up/0064_create_locations.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0064_create_locations.sql"),
                include_str!("../sql/postgres/down/0064_create_locations.sql"),
                include_str!("../sql/mysql/down/0064_create_locations.sql"),
            ),
        )
        .await
    }
}
