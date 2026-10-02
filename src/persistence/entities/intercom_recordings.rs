use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "intercom_recordings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub space_id: String,
    pub case_id: String,
    pub user_id: String,
    pub object_key: String,
    pub source_content_type: String,
    pub final_content_type: String,
    pub byte_size: i64,
    pub started_at: String,
    pub ended_at: String,
    pub transcription_status: String,
    pub created_at: String,
    pub failed_reason: Option<String>,
    pub audio_deleted_at: Option<String>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
