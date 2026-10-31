use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "case_map_areas")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub case_id: String,
    pub area_type: String,
    pub status: String,
    pub title: String,
    pub description: Option<String>,
    pub task_id: Option<String>,
    pub clue_id: Option<String>,
    pub version: i32,
    pub created_by_user_id: String,
    pub reviewed_by_user_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::cases::Entity",
        from = "Column::CaseId",
        to = "super::cases::Column::Id"
    )]
    Case,
}

impl Related<super::cases::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Case.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
