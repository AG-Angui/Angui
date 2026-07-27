use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub case_id: String,
    pub source_clue_id: Option<String>,
    pub title: String,
    pub objective: String,
    pub area_text: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub due_at: String,
    pub background: String,
    pub risk_level: String,
    pub risk_notes: String,
    pub safety_briefing: String,
    pub expected_feedback: String,
    pub status: String,
    pub result_summary: Option<String>,
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
    #[sea_orm(
        belongs_to = "super::clues::Entity",
        from = "Column::SourceClueId",
        to = "super::clues::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    SourceClue,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatedByUserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Creator,
}

impl Related<super::cases::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Case.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
