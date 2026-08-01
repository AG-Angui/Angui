use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "clue_drafts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub case_id: String,
    pub status: String,
    pub content: String,
    pub source_type: String,
    pub raw_record_reference: Option<String>,
    pub uncertainty_notice: String,
    pub template_version: String,
    pub provider_model: Option<String>,
    pub degradation_status: String,
    pub candidate_json: String,
    pub review_status: String,
    pub reviewed_by_user_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub review_reason: Option<String>,
    pub version: i32,
    pub promoted_clue_id: Option<String>,
    pub created_by_user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::cases::Entity",
        from = "Column::CaseId",
        to = "super::cases::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Case,
}

impl ActiveModelBehavior for ActiveModel {}
