use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "intake_session_answers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub session_id: String,
    pub field_code: String,
    pub raw_answer: String,
    pub candidate_value: String,
    pub source: String,
    pub status: String,
    pub generated_at: String,
    pub model: Option<String>,
    pub template_version: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::intake_sessions::Entity",
        from = "Column::SessionId",
        to = "super::intake_sessions::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Session,
}

impl ActiveModelBehavior for ActiveModel {}
