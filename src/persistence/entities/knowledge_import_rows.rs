use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "knowledge_import_rows")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub batch_id: String,
    pub row_number: i32,
    pub raw_data_json: String,
    pub normalized_data_json: String,
    pub status: String,
    pub error_message: Option<String>,
    pub knowledge_item_id: Option<String>,
    pub created_at: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
