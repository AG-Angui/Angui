use sea_orm_migration::prelude::*;

use crate::{execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0017_add_two_phase_intake_questions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0017_add_two_phase_intake_questions.sql"),
                include_str!("../sql/postgres/up/0017_add_two_phase_intake_questions.sql"),
                include_str!("../sql/mysql/up/0017_add_two_phase_intake_questions.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0017_add_two_phase_intake_questions.sql"),
                include_str!("../sql/postgres/down/0017_add_two_phase_intake_questions.sql"),
                include_str!("../sql/mysql/down/0017_add_two_phase_intake_questions.sql"),
            ),
        )
        .await
    }
}
