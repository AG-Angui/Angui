use sea_orm_migration::prelude::*;

use crate::{ensure_rollback_is_safe, execute_script, sql_for_backend};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0021_create_user_profiles_and_elder_profile_revisions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!(
                    "../sql/sqlite/up/0021_create_user_profiles_and_elder_profile_revisions.sql"
                ),
                include_str!(
                    "../sql/postgres/up/0021_create_user_profiles_and_elder_profile_revisions.sql"
                ),
                include_str!(
                    "../sql/mysql/up/0021_create_user_profiles_and_elder_profile_revisions.sql"
                ),
            ),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_rollback_is_safe(manager, &[
            ("user profile settings exist", "SELECT 1 FROM user_profiles WHERE avatar_reference IS NOT NULL OR preferences_json <> '{\"locale\":\"zh-CN\",\"reduced_motion\":false}' LIMIT 1"),
            ("elder profile revisions exist", "SELECT 1 FROM elder_profile_revisions LIMIT 1"),
        ]).await?;
        execute_script(
            manager,
            sql_for_backend(
                manager,
                include_str!(
                    "../sql/sqlite/down/0021_drop_user_profiles_and_elder_profile_revisions.sql"
                ),
                include_str!(
                    "../sql/postgres/down/0021_drop_user_profiles_and_elder_profile_revisions.sql"
                ),
                include_str!(
                    "../sql/mysql/down/0021_drop_user_profiles_and_elder_profile_revisions.sql"
                ),
            ),
        )
        .await
    }
}
