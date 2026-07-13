use sea_orm_migration::prelude::*;

use crate::{execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0004_create_audit_events"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = sql_for_backend(
            manager,
            include_str!("../sql/sqlite/up/0004_create_audit_events.sql"),
            include_str!("../sql/postgres/up/0004_create_audit_events.sql"),
            include_str!("../sql/mysql/up/0004_create_audit_events.sql"),
        );
        execute_script(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = sql_for_backend(
            manager,
            include_str!("../sql/sqlite/down/0004_drop_audit_events.sql"),
            include_str!("../sql/postgres/down/0004_drop_audit_events.sql"),
            include_str!("../sql/mysql/down/0004_drop_audit_events.sql"),
        );
        execute_script(manager, sql).await
    }
}
