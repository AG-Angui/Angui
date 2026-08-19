use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "space_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub space_id: String,
    pub case_id: String,
    pub event_type: String,
    pub version: i32,
    pub visibility_scope: String,
    pub payload_json: String,
    pub occurred_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::collaboration_spaces::Entity",
        from = "Column::SpaceId",
        to = "super::collaboration_spaces::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Space,
}

impl Related<super::collaboration_spaces::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Space.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
