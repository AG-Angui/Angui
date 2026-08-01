use sea_orm_migration::prelude::*;

use crate::{execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0035_add_clue_promotion"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0035_add_clue_promotion.sql"),
                include_str!("../sql/postgres/up/0035_add_clue_promotion.sql"),
                include_str!("../sql/mysql/up/0035_add_clue_promotion.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0035_remove_clue_promotion.sql"),
                include_str!("../sql/postgres/down/0035_remove_clue_promotion.sql"),
                include_str!("../sql/mysql/down/0035_remove_clue_promotion.sql"),
            ),
        )
        .await
    }
}
