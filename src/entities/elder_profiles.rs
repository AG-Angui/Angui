use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "elder_profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub case_id: String,
    pub display_name: String,
    pub age: Option<i16>,
    pub gender: Option<String>,
    pub physical_description: Option<String>,
    pub clothing_description: Option<String>,
    pub health_notes: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_seen_location: Option<String>,
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
