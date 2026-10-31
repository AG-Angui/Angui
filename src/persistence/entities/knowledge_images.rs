use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "knowledge_images")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub knowledge_item_id: String,
    pub storage_path: String,
    pub mime_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub metadata_json: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
