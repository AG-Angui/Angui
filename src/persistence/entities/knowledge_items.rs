use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "knowledge_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub knowledge_base_id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub category: String,
    pub category_id: Option<String>,
    pub keywords_json: String,
    pub metadata_json: String,
    pub source_name: String,
    pub source_url: Option<String>,
    pub visibility: String,
    pub status: String,
    pub effective_at: String,
    pub withdrawn_at: Option<String>,
    pub previous_version_id: Option<String>,
    pub version: i32,
    pub content_hash: String,
    pub embedding_json: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dimension: Option<i32>,
    pub embedding_status: String,
    pub embedding_generated_at: Option<String>,
    pub embedding_content_hash: Option<String>,
    pub created_by_user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
