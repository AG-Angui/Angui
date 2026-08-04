use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0038_create_learning_center"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let statements: Vec<_> = sql_for_backend(
            manager,
            include_str!("../sql/sqlite/up/0038_create_learning_center.sql"),
            include_str!("../sql/postgres/up/0038_create_learning_center.sql"),
            include_str!("../sql/mysql/up/0038_create_learning_center.sql"),
        )
        .split("-- statement-break")
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
        .collect();

        let objects = [
            ("learning_resources", "learning_resources", false),
            ("idx_learning_resources_visible", "learning_resources", true),
            ("learning_questions", "learning_questions", false),
            ("idx_learning_questions_visible", "learning_questions", true),
            (
                "learning_question_answers",
                "learning_question_answers",
                false,
            ),
            (
                "idx_learning_question_answers_user_created",
                "learning_question_answers",
                true,
            ),
        ];

        for ((name, table, is_index), statement) in objects.into_iter().zip(statements) {
            let exists = if is_index {
                manager.has_index(table, name).await?
            } else {
                manager.has_table(name).await?
            };
            if !exists {
                manager
                    .get_connection()
                    .execute_unprepared(statement)
                    .await?;
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(
            manager,
            &[(
                "learning question answers exist",
                "SELECT 1 FROM learning_question_answers LIMIT 1",
            )],
        )
        .await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!("../sql/sqlite/down/0038_drop_learning_center.sql"),
                include_str!("../sql/postgres/down/0038_drop_learning_center.sql"),
                include_str!("../sql/mysql/down/0038_drop_learning_center.sql"),
            ),
        )
        .await
    }
}
