use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "ai_executions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub owner_user_id: String,
    pub intake_session_id: Option<String>,
    pub workflow: String,
    pub stage: String,
    pub status: String,
    pub failure_kind: Option<String>,
    pub result_status: Option<String>,
    pub fallback_used: bool,
    pub last_event_id: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::OwnerUserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Owner,
    #[sea_orm(
        belongs_to = "super::intake_sessions::Entity",
        from = "Column::IntakeSessionId",
        to = "super::intake_sessions::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    IntakeSession,
}

impl ActiveModelBehavior for ActiveModel {}
