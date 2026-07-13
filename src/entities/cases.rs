use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "cases")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub case_code: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::elder_profiles::Entity")]
    ElderProfile,
    #[sea_orm(has_many = "super::clues::Entity")]
    Clues,
    #[sea_orm(has_many = "super::audit_events::Entity")]
    AuditEvents,
}

impl Related<super::elder_profiles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ElderProfile.def()
    }
}

impl Related<super::clues::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Clues.def()
    }
}

impl Related<super::audit_events::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AuditEvents.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
