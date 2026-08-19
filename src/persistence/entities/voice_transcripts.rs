use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "voice_transcripts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub voice_report_id: String,
    pub content: String,
    pub provider: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::voice_reports::Entity",
        from = "Column::VoiceReportId",
        to = "super::voice_reports::Column::Id"
    )]
    VoiceReport,
}

impl Related<super::voice_reports::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VoiceReport.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
