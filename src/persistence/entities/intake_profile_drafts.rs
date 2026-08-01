use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "intake_profile_drafts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub session_id: String,
    pub version: i32,
    pub parent_draft_id: Option<String>,
    pub profile_json: String,
    pub field_metadata_json: String,
    pub status: String,
    pub degradation_status: String,
    pub provider_model: Option<String>,
    pub template_version: String,
    pub generated_at: String,
    pub confirmed_by_user_id: Option<String>,
    pub confirmed_at: Option<String>,
    pub created_at: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
