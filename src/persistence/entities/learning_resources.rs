use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "learning_resources")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub resource_type: String,
    pub tags_json: String,
    pub source_name: String,
    pub source_url: Option<String>,
    pub version: i32,
    pub visibility: String,
    pub status: String,
    pub effective_at: String,
    pub withdrawn_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
