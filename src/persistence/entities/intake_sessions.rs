use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "intake_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub created_by_user_id: String,
    pub case_id: Option<String>,
    pub question_set_version: i32,
    pub status: String,
    pub answers_json: String,
    pub assessment_json: String,
    pub structured_answers_json: String,
    pub ai_initial_review_status: String,
    pub ai_initial_review_json: String,
    pub ai_initial_review_profile_json: Option<String>,
    pub ai_initial_reviewed_at: Option<String>,
    pub confirmed_by_user_id: Option<String>,
    pub confirmed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatedByUserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Creator,
    #[sea_orm(
        belongs_to = "super::cases::Entity",
        from = "Column::CaseId",
        to = "super::cases::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Case,
}

impl ActiveModelBehavior for ActiveModel {}
