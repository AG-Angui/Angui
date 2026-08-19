use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "knowledge_import_batches")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub knowledge_base_id: String,
    pub file_name: String,
    pub status: String,
    pub total_rows: i32,
    pub valid_rows: i32,
    pub invalid_rows: i32,
    pub created_by_user_id: String,
    pub confirmed_by_user_id: Option<String>,
    pub created_at: String,
    pub confirmed_at: Option<String>,
    pub updated_at: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
