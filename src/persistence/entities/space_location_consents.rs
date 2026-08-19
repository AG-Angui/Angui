use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "space_location_consents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub space_id: String,
    pub user_id: String,
    pub member_id: String,
    pub consent_version: String,
    pub granted_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::space_members::Entity",
        from = "Column::MemberId",
        to = "super::space_members::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Member,
    #[sea_orm(
        belongs_to = "super::collaboration_spaces::Entity",
        from = "Column::SpaceId",
        to = "super::collaboration_spaces::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Space,
}

impl Related<super::space_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Member.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
