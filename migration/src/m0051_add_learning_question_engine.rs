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
                include_str!("../sql/sqlite/up/0051_add_learning_question_engine.sql"),
                include_str!("../sql/postgres/up/0051_add_learning_question_engine.sql"),
                include_str!("../sql/mysql/up/0051_add_learning_question_engine.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0051_remove_learning_question_engine.sql"),
                include_str!("../sql/postgres/down/0051_remove_learning_question_engine.sql"),
                include_str!("../sql/mysql/down/0051_remove_learning_question_engine.sql"),
            ),
        )
        .await
    }
}
