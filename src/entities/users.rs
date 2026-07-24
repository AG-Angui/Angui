use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub email: String,
    pub display_name: String,
    pub account_type: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::auth_sessions::Entity")]
    AuthSessions,
    #[sea_orm(has_many = "super::case_memberships::Entity")]
    CaseMemberships,
    #[sea_orm(has_many = "super::user_global_capabilities::Entity")]
    GlobalCapabilities,
}

impl Related<super::auth_sessions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AuthSessions.def()
    }
}

impl Related<super::case_memberships::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CaseMemberships.def()
    }
}

impl Related<super::user_global_capabilities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GlobalCapabilities.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
