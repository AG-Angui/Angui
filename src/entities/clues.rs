use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "clues")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub case_id: String,
    pub status: String,
    pub source: String,
    pub source_type: String,
    pub content: String,
    pub raw_record_reference: Option<String>,
    pub occurred_at: Option<String>,
    pub reported_at: String,
    pub confirmed_at: Option<String>,
    pub location_text: Option<String>,
    pub location_precision: Option<String>,
    pub next_action: Option<String>,
    pub linked_task_reference: Option<String>,
    pub related_clue_id: Option<String>,
    pub relationship_type: Option<String>,
    pub review_reason: Option<String>,
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

impl Related<super::cases::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Case.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
