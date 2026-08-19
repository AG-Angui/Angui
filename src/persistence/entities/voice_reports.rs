use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "voice_reports")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub space_id: String,
    pub case_id: String,
    pub reporter_id: String,
    pub object_key: String,
    pub content_type: String,
    pub byte_size: i64,
    pub status: String,
    pub created_at: String,
    pub failed_reason: Option<String>,
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
