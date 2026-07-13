use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "audit_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub case_id: Option<String>,
    pub actor: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub metadata_json: Option<String>,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::cases::Entity",
        from = "Column::CaseId",
        to = "super::cases::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Case,
}

impl Related<super::cases::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Case.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
