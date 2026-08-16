use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "space_messages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub space_id: String,
    pub sender_id: String,
    pub message_type: String,
    pub content: String,
    pub sent_at: String,
    pub recalled_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::collaboration_spaces::Entity",
        from = "Column::SpaceId",
        to = "super::collaboration_spaces::Column::Id"
    )]
    Space,
}
impl Related<super::collaboration_spaces::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Space.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}
