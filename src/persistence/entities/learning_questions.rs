use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "learning_questions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub source_resource_id: String,
    pub prompt: String,
    pub question_type: String,
    pub difficulty: String,
    pub tags_json: String,
    pub options_json: String,
    pub correct_option_id: String,
    pub explanation: String,
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
