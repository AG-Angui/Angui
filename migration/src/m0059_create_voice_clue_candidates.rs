use crate::{execute_script, sql_for_backend};
use sea_orm_migration::prelude::*;

pub struct Migration;
impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0059_create_voice_clue_candidates"
    }
}
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0059_create_voice_clue_candidates.sql"),
                include_str!("../sql/postgres/up/0059_create_voice_clue_candidates.sql"),
                include_str!("../sql/mysql/up/0059_create_voice_clue_candidates.sql"),
            ),
        )
        .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0059_drop_voice_clue_candidates.sql"),
                include_str!("../sql/postgres/down/0059_drop_voice_clue_candidates.sql"),
                include_str!("../sql/mysql/down/0059_drop_voice_clue_candidates.sql"),
            ),
        )
        .await
    }
}
