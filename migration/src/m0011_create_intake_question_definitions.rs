use sea_orm_migration::prelude::*;

use crate::{execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0011_create_intake_question_definitions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = sql_for_backend(
            manager,
            include_str!("../sql/sqlite/up/0011_create_intake_question_definitions.sql"),
            include_str!("../sql/postgres/up/0011_create_intake_question_definitions.sql"),
            include_str!("../sql/mysql/up/0011_create_intake_question_definitions.sql"),
        );
        execute_script(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = sql_for_backend(
            manager,
            include_str!("../sql/sqlite/down/0011_drop_intake_question_definitions.sql"),
            include_str!("../sql/postgres/down/0011_drop_intake_question_definitions.sql"),
            include_str!("../sql/mysql/down/0011_drop_intake_question_definitions.sql"),
        );
        execute_script(manager, sql).await
    }
}
