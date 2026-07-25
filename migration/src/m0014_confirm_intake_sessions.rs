use sea_orm_migration::prelude::*;

use crate::{execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0014_confirm_intake_sessions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/up/0014_confirm_intake_sessions.sql"),
                include_str!("../sql/postgres/up/0014_confirm_intake_sessions.sql"),
                include_str!("../sql/mysql/up/0014_confirm_intake_sessions.sql"),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0014_confirm_intake_sessions.sql"),
                include_str!("../sql/postgres/down/0014_confirm_intake_sessions.sql"),
                include_str!("../sql/mysql/down/0014_confirm_intake_sessions.sql"),
            ),
        )
        .await
    }
}
