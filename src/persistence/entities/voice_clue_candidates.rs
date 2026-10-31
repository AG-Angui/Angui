use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "voice_clue_candidates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub case_id: String,
    pub voice_report_id: Option<String>,
    pub intercom_recording_id: Option<String>,
    pub submitted_by_user_id: String,
    pub discoverer_user_id: Option<String>,
    pub source_type: String,
    pub ai_generated: bool,
    pub transcript_text: Option<String>,
    pub candidate_json: String,
    pub asr_version: Option<String>,
    pub model_version: Option<String>,
    pub status: String,
    pub retry_count: i32,
    pub failure_reason: Option<String>,
    pub promoted_clue_id: Option<String>,
    pub clue_draft_id: Option<String>,
    pub returned_for_revision: bool,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
