use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "learning_question_resources")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub question_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub resource_id: String,
    pub position: i32,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
