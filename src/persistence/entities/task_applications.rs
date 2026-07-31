use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "task_applications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub task_id: String,
    pub volunteer_user_id: String,
    pub status: String,
    pub note: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub review_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
